//! Allineamento: scansione, piano, esecuzione.
//!
//! Ogni azione viene scritta nel journal prima di toccare i file e tolta,
//! insieme all'aggiornamento della base, nella stessa transazione quando è
//! finita. Se l'allineamento si interrompe, la base resta quella di prima per
//! le azioni non finite: al giro successivo il piano ritrova le stesse
//! differenze, meno quelle già sistemate (vedi `doc/formato-dati.md`).

use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::archive::{Archive, Side};
use crate::copy::safe_copy;
use crate::error::{Error, Result};
use crate::hash::hash_file;
use crate::index::{BaseEntry, Kind, Tx};
use crate::paths::{self, key, native};
use crate::plan::{self, Action, Answer, Plan, Question};
use crate::scan::{self, Scan, mtime_ns, ns_system_time, same_mtime};

/// Ogni quanto riscrivere l'indice sui dischi durante un allineamento lungo.
const SAVE_EVERY: Duration = Duration::from_secs(30);

/// Avanzamento dell'esecuzione.
#[derive(Debug, Clone)]
pub struct Progress<'a> {
    pub done: usize,
    pub total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    /// Percorso dell'azione in corso.
    pub current: &'a str,
}

/// Un'azione saltata e il motivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub path: String,
    pub reason: String,
}

/// Risultato dell'esecuzione.
#[derive(Debug, Default)]
pub struct Report {
    pub done: usize,
    pub bytes_copied: u64,
    /// Azioni saltate perché il file è cambiato, è bloccato o non c'è più:
    /// si riprendono al giro successivo.
    pub skipped: Vec<Skipped>,
}

/// Esito di una singola azione.
enum Outcome {
    Done {
        put: Option<BaseEntry>,
        forget: Option<String>,
        event: Option<(&'static str, Side)>,
    },
    Skipped(String),
}

impl Archive {
    /// Scansiona entrambi i dischi.
    pub fn scan(&self) -> Result<[Scan; 2]> {
        Ok([
            scan::scan(self.disk(Side::A).root())?,
            scan::scan(self.disk(Side::B).root())?,
        ])
    }

    /// Confronta i dischi con la base e restituisce il piano.
    pub fn plan(&self) -> Result<Plan> {
        let [a, b] = self.scan()?;
        let base = self.index().base()?;
        let roots = [
            self.disk(Side::A).root().to_path_buf(),
            self.disk(Side::B).root().to_path_buf(),
        ];
        let mut hash = |side: Side, path: &str| hash_file(&native(&roots[side as usize], path));
        Ok(plan::plan(&base, &a, &b, &mut hash)?)
    }

    /// Azioni per la risposta a una domanda.
    pub fn resolve(&self, question: &Question, answer: Answer) -> Vec<Action> {
        let b = self.disk(Side::B);
        let root_b = b.root().to_path_buf();
        plan::resolve(question, answer, b.label(), &|p| {
            native(&root_b, p).exists()
        })
    }

    /// Esegue le azioni e riscrive l'indice su entrambi i dischi.
    ///
    /// Prima di iniziare controlla lo spazio libero. Un'azione su un file
    /// cambiato, bloccato o sparito viene saltata; un disco scollegato
    /// interrompe tutto con un errore (il journal ricorda l'azione in corso).
    pub fn apply(
        &mut self,
        actions: &[Action],
        progress: &mut dyn FnMut(&Progress),
    ) -> Result<Report> {
        self.check_space(actions)?;
        let bytes_total = actions
            .iter()
            .filter_map(Action::copy_size)
            .map(|(_, n)| n)
            .sum();
        let mut report = Report::default();
        let mut last_save = Instant::now();

        for (i, action) in actions.iter().enumerate() {
            let current = action_path(action);
            progress(&Progress {
                done: i,
                total: actions.len(),
                bytes_done: report.bytes_copied,
                bytes_total,
                current,
            });
            let id = self
                .index_mut()
                .update(|tx| tx.journal_begin(action_name(action), &format!("{action:?}")))?;
            let mut bytes = 0;
            let outcome = match self.execute(action, &mut |n| bytes += n) {
                Ok(o) => o,
                Err(e) if self.disks_present() => Outcome::Skipped(e.to_string()),
                Err(e) => {
                    let _ = self.save();
                    return Err(e);
                }
            };
            report.bytes_copied += bytes;
            let disk_ids = [
                self.disk(Side::A).id().to_string(),
                self.disk(Side::B).id().to_string(),
            ];
            self.index_mut().update(|tx: &Tx| {
                tx.journal_end(id)?;
                match &outcome {
                    Outcome::Done { put, forget, event } => {
                        if let Some(k) = forget {
                            tx.forget(k)?;
                        }
                        if let Some(e) = put {
                            tx.put(&key(&e.path), e)?;
                        }
                        if let Some((kind, side)) = event {
                            tx.log(kind, Some(current), Some(&disk_ids[*side as usize]), None)?;
                        }
                    }
                    Outcome::Skipped(reason) => {
                        tx.log("skipped", Some(current), None, Some(reason))?;
                    }
                }
                Ok(())
            })?;
            match outcome {
                Outcome::Done { .. } => report.done += 1,
                Outcome::Skipped(reason) => report.skipped.push(Skipped {
                    path: current.to_string(),
                    reason,
                }),
            }
            if last_save.elapsed() > SAVE_EVERY {
                self.save()?;
                last_save = Instant::now();
            }
        }
        progress(&Progress {
            done: actions.len(),
            total: actions.len(),
            bytes_done: report.bytes_copied,
            bytes_total,
            current: "",
        });
        let now = crate::now_rfc3339();
        self.index_mut()
            .update(|tx| tx.set_meta("last_sync", Some(&now)))?;
        self.save()?;
        Ok(report)
    }

    fn root(&self, side: Side) -> PathBuf {
        self.disk(side).root().to_path_buf()
    }

    fn path(&self, side: Side, rel: &str) -> PathBuf {
        native(self.disk(side).root(), rel)
    }

    /// Entrambi i dischi sono ancora collegati.
    fn disks_present(&self) -> bool {
        Side::BOTH.iter().all(|s| self.disk(*s).data_dir().is_dir())
    }

    fn check_space(&self, actions: &[Action]) -> Result<()> {
        for side in Side::BOTH {
            let needed: u64 = actions
                .iter()
                .filter_map(Action::copy_size)
                .filter(|(s, _)| *s == side)
                .map(|(_, n)| n)
                .sum();
            if needed == 0 {
                continue;
            }
            let available = fs4::available_space(self.root(side))?;
            if needed > available {
                return Err(Error::NoSpace {
                    label: self.disk(side).label().to_string(),
                    needed,
                    available,
                });
            }
        }
        Ok(())
    }

    fn execute(&self, action: &Action, progress: &mut dyn FnMut(u64)) -> Result<Outcome> {
        Ok(match action {
            Action::Copy {
                from,
                src,
                dest_path,
                hash,
            } => {
                let src_path = self.path(*from, &src.path);
                let meta = fs::metadata(&src_path)?;
                if meta.len() != src.size || !same_mtime(mtime_ns(&meta)?, src.mtime) {
                    return Ok(Outcome::Skipped("il file è cambiato".into()));
                }
                let to = from.other();
                let copied = safe_copy(
                    &src_path,
                    &self.path(to, dest_path),
                    hash.as_deref(),
                    progress,
                )?;
                Outcome::Done {
                    put: Some(BaseEntry {
                        path: src.path.clone(),
                        kind: Kind::File,
                        size: copied.size,
                        mtime: copied.mtime,
                        hash: Some(copied.hash),
                    }),
                    forget: (key(dest_path) != key(&src.path)).then(|| key(dest_path)),
                    event: Some(("copied", to)),
                }
            }
            Action::Move {
                side,
                from,
                to,
                old_key,
                entry,
            } => {
                let from_path = self.path(*side, from);
                let to_path = self.path(*side, to);
                if !from_path.is_file() {
                    return Ok(Outcome::Skipped("il file non c'è più".into()));
                }
                if key(from) == key(to) {
                    // Solo maiuscole: in due passi, così cambia davvero
                    // anche su exFAT.
                    let tmp = to_path.with_file_name(format!(
                        ".{}{}",
                        paths::file_name(to),
                        paths::RENAME_SUFFIX
                    ));
                    fs::rename(&from_path, &tmp)?;
                    fs::rename(&tmp, &to_path)?;
                } else {
                    if to_path.exists() {
                        return Ok(Outcome::Skipped("la destinazione esiste già".into()));
                    }
                    if let Some(parent) = to_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::rename(&from_path, &to_path)?;
                    self.prune(*side, from);
                }
                Outcome::Done {
                    put: Some(entry.clone()),
                    forget: (*old_key != key(&entry.path)).then(|| old_key.clone()),
                    event: Some(("moved", *side)),
                }
            }
            Action::Delete { side, expect, key } => {
                let path = self.path(*side, &expect.path);
                let meta = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        return Ok(Outcome::Done {
                            put: None,
                            forget: Some(key.clone()),
                            event: None,
                        });
                    }
                    Err(e) => return Err(e.into()),
                };
                if meta.len() != expect.size || !same_mtime(mtime_ns(&meta)?, expect.mtime) {
                    return Ok(Outcome::Skipped("il file è cambiato".into()));
                }
                fs::remove_file(&path)?;
                self.prune(*side, &expect.path);
                Outcome::Done {
                    put: None,
                    forget: Some(key.clone()),
                    event: Some(("deleted", *side)),
                }
            }
            Action::SetMtime { side, entry } => {
                OpenOptions::new()
                    .write(true)
                    .open(self.path(*side, &entry.path))?
                    .set_modified(ns_system_time(entry.mtime))?;
                Outcome::Done {
                    put: Some(entry.clone()),
                    forget: None,
                    event: None,
                }
            }
            Action::MakeDir { side, path } => {
                fs::create_dir_all(self.path(*side, path))?;
                Outcome::Done {
                    put: Some(BaseEntry {
                        path: path.clone(),
                        kind: Kind::Dir,
                        size: 0,
                        mtime: 0,
                        hash: None,
                    }),
                    forget: None,
                    event: Some(("folder_created", *side)),
                }
            }
            Action::RemoveDir { side, path } => {
                let dir = self.path(*side, path);
                if dir.exists() {
                    fs::remove_dir(&dir)?;
                }
                self.prune(*side, path);
                Outcome::Done {
                    put: None,
                    forget: Some(key(path)),
                    event: Some(("folder_deleted", *side)),
                }
            }
            Action::Rename { side, from, to } => {
                let to_path = self.path(*side, to);
                if to_path.exists() {
                    return Ok(Outcome::Skipped("la destinazione esiste già".into()));
                }
                fs::rename(self.path(*side, from), &to_path)?;
                Outcome::Done {
                    put: None,
                    forget: None,
                    event: Some(("renamed", *side)),
                }
            }
            Action::Record { entry } => Outcome::Done {
                put: Some(entry.clone()),
                forget: None,
                event: None,
            },
            Action::Forget { key } => Outcome::Done {
                put: None,
                forget: Some(key.clone()),
                event: None,
            },
        })
    }

    /// Dopo aver tolto `rel` dal disco `side`, toglie le cartelle rimaste
    /// vuote che sull'altro disco non esistono più: rispecchia una cartella
    /// spostata o cancellata dall'utente.
    fn prune(&self, side: Side, rel: &str) {
        let mut dir = paths::parent(rel);
        while let Some(d) = dir {
            if self.path(side.other(), d).exists() {
                break;
            }
            if fs::remove_dir(self.path(side, d)).is_err() {
                break;
            }
            dir = paths::parent(d);
        }
    }
}

fn action_name(action: &Action) -> &'static str {
    match action {
        Action::Copy { .. } => "copy",
        Action::Move { .. } => "move",
        Action::Delete { .. } => "delete",
        Action::SetMtime { .. } => "set_mtime",
        Action::MakeDir { .. } => "make_dir",
        Action::RemoveDir { .. } => "remove_dir",
        Action::Rename { .. } => "rename",
        Action::Record { .. } => "record",
        Action::Forget { .. } => "forget",
    }
}

fn action_path(action: &Action) -> &str {
    match action {
        Action::Copy { src, .. } => &src.path,
        Action::Move { to, .. } => to,
        Action::Delete { expect, .. } => &expect.path,
        Action::SetMtime { entry, .. } | Action::Record { entry } => &entry.path,
        Action::MakeDir { path, .. } | Action::RemoveDir { path, .. } => path,
        Action::Rename { from, .. } => from,
        Action::Forget { key } => key,
    }
}

#[cfg(test)]
mod tests;
