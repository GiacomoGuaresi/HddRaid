//! Controllo completo: rilegge tutti i file di entrambi i dischi e ne
//! confronta l'hash con quello della base, per trovare i file rovinati (bit
//! rot) e ripararli dalla copia buona.
//!
//! I due dischi si leggono in parallelo. Il controllo si può interrompere:
//! ogni file verificato viene segnato nell'indice, e il controllo successivo
//! riprende dai file non ancora verificati in questo giro.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};

use crate::archive::{Archive, Side};
use crate::copy::safe_copy;
use crate::disk::DATA_DIR;
use crate::error::Result;
use crate::hash::hash_file;
use crate::index::{BaseEntry, Kind};
use crate::paths::native;
use crate::scan::{mtime_ns, same_mtime};
use crate::sync::{Progress, Report, Skipped};

/// Ogni quanto scrivere le verifiche nell'indice durante il controllo.
const FLUSH_EVERY: Duration = Duration::from_secs(5);
/// Ogni quanto riscrivere l'indice sui dischi.
const SAVE_EVERY: Duration = Duration::from_secs(60);

/// Stato di un file su un disco.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Health {
    /// Hash uguale alla base.
    Ok,
    /// Stessa dimensione e data, hash diverso: il file si è rovinato.
    Corrupt,
    /// Dimensione o data diverse: è una modifica, se ne occupa l'allineamento.
    Changed,
    /// Il file non c'è.
    Missing,
}

/// Avanzamento del controllo.
#[derive(Debug, Clone)]
pub struct CheckProgress {
    pub checked: usize,
    pub total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

/// Un file rovinato su un disco e integro sull'altro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repair {
    pub key: String,
    pub path: String,
    /// Il disco con la copia rovinata.
    pub bad: Side,
    /// Hash giusto, quello della base.
    pub hash: String,
}

/// Risultato del controllo.
#[derive(Debug, Default)]
pub struct CheckReport {
    /// `false` se è stato interrotto: riprenderà dai file mancanti.
    pub completed: bool,
    /// File verificati e integri in questo giro, su entrambi i dischi.
    pub ok: usize,
    /// File da riparare dalla copia buona.
    pub repairs: Vec<Repair>,
    /// File rovinati su un disco e non integri sull'altro: non si possono
    /// riparare.
    pub lost: Vec<String>,
    /// File cambiati o mancanti: se ne occupa l'allineamento.
    pub changed: Vec<String>,
}

impl Archive {
    /// `true` se l'ultimo controllo completo è più vecchio di `every`, o non
    /// è mai stato fatto, o ce n'è uno da riprendere.
    pub fn check_due(&self, every: Duration) -> Result<bool> {
        if self.index().meta("full_check_started")?.is_some() {
            return Ok(true);
        }
        let Some(last) = self.index().meta("last_full_check")? else {
            return Ok(true);
        };
        let Ok(last) = DateTime::parse_from_rfc3339(&last) else {
            return Ok(true);
        };
        let age = Utc::now().signed_duration_since(last);
        Ok(age.to_std().unwrap_or_default() >= every)
    }

    /// Esegue (o riprende) il controllo completo. Si ferma quando `cancel`
    /// diventa `true`, salvando i progressi.
    pub fn full_check(
        &mut self,
        progress: &mut dyn FnMut(&CheckProgress),
        cancel: &AtomicBool,
    ) -> Result<CheckReport> {
        let started = match self.index().meta("full_check_started")? {
            Some(s) => s,
            None => {
                let now = crate::now_rfc3339();
                self.index_mut().update(|tx| {
                    tx.set_meta("full_check_started", Some(&now))?;
                    tx.log("full_check_started", None, None, None)
                })?;
                self.save()?;
                now
            }
        };

        let base: BTreeMap<String, BaseEntry> = self
            .index()
            .base()?
            .into_iter()
            .filter(|(_, e)| e.kind == Kind::File && e.hash.is_some())
            .collect();
        let disk_ids = Side::BOTH.map(|s| self.disk(s).id().to_string());
        let roots = Side::BOTH.map(|s| self.disk(s).root().to_path_buf());
        let todo = Side::BOTH.map(|s| -> Result<Vec<(String, BaseEntry)>> {
            let done = self
                .index()
                .verified_since(&disk_ids[s as usize], &started)?;
            Ok(base
                .iter()
                .filter(|(k, _)| !done.contains(*k))
                .map(|(k, e)| (k.clone(), e.clone()))
                .collect())
        });
        let [todo_a, todo_b] = todo;
        let todo = [todo_a?, todo_b?];
        let total = todo[0].len() + todo[1].len();
        let bytes_total = todo.iter().flatten().map(|(_, e)| e.size).sum();

        let mut report = CheckReport::default();
        let mut corrupt: [HashSet<String>; 2] = Default::default();
        let mut pending: Vec<(String, Side)> = Vec::new();
        let mut state = CheckProgress {
            checked: 0,
            total,
            bytes_done: 0,
            bytes_total,
        };
        progress(&state);

        // Un disco scollegato interrompe il controllo, come `cancel`.
        let gone = AtomicBool::new(false);
        let (tx, rx) = mpsc::channel::<(Side, String, Health, u64)>();
        let result: Result<()> = std::thread::scope(|scope| {
            for side in Side::BOTH {
                let tx = tx.clone();
                let list = &todo[side as usize];
                let root = &roots[side as usize];
                let gone = &gone;
                scope.spawn(move || {
                    for (key, entry) in list {
                        if cancel.load(Ordering::Relaxed) || gone.load(Ordering::Relaxed) {
                            break;
                        }
                        let health = check_file(&native(root, &entry.path), entry);
                        if health == Health::Missing && !root.join(DATA_DIR).is_dir() {
                            gone.store(true, Ordering::Relaxed);
                            break;
                        }
                        if tx.send((side, key.clone(), health, entry.size)).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(tx);

            let mut last_flush = Instant::now();
            let mut last_save = Instant::now();
            for (side, key, health, size) in rx {
                state.checked += 1;
                state.bytes_done += size;
                match health {
                    Health::Ok => pending.push((key, side)),
                    Health::Corrupt => {
                        corrupt[side as usize].insert(key);
                    }
                    Health::Changed | Health::Missing => {
                        report.changed.push(base[&key].path.clone())
                    }
                }
                if last_flush.elapsed() > FLUSH_EVERY {
                    self.flush_verified(&mut pending, &disk_ids)?;
                    last_flush = Instant::now();
                }
                if last_save.elapsed() > SAVE_EVERY {
                    self.save()?;
                    last_save = Instant::now();
                }
                progress(&state);
            }
            Ok(())
        });
        result?;
        self.flush_verified(&mut pending, &disk_ids)?;

        if gone.load(Ordering::Relaxed) {
            // L'indice si salva dove si può: il controllo riprenderà dal
            // disco con le verifiche più recenti.
            let _ = self.save();
            return Ok(report);
        }
        if cancel.load(Ordering::Relaxed) && state.checked < total {
            self.save()?;
            return Ok(report);
        }

        let verified = [
            self.index().verified_since(&disk_ids[0], &started)?,
            self.index().verified_since(&disk_ids[1], &started)?,
        ];
        for side in Side::BOTH {
            for key in &corrupt[side as usize] {
                let entry = &base[key];
                if verified[side.other() as usize].contains(key) {
                    report.repairs.push(Repair {
                        key: key.clone(),
                        path: entry.path.clone(),
                        bad: side,
                        hash: entry.hash.clone().unwrap_or_default(),
                    });
                } else if side == Side::A || !corrupt[0].contains(key) {
                    report.lost.push(entry.path.clone());
                }
            }
        }
        report.repairs.sort_by(|a, b| a.key.cmp(&b.key));
        report.lost.sort();
        report.changed.sort();
        report.changed.dedup();
        report.ok = base
            .keys()
            .filter(|k| verified[0].contains(*k) && verified[1].contains(*k))
            .count();
        report.completed = true;

        let now = crate::now_rfc3339();
        let details = format!(
            "ok={} repairs={} lost={}",
            report.ok,
            report.repairs.len(),
            report.lost.len()
        );
        self.index_mut().update(|tx| {
            tx.set_meta("full_check_started", None)?;
            tx.set_meta("last_full_check", Some(&now))?;
            tx.log("full_check_completed", None, None, Some(&details))
        })?;
        self.save()?;
        Ok(report)
    }

    /// Ripara i file rovinati copiandoli dal disco con la copia buona, con
    /// verifica dell'hash.
    pub fn repair(
        &mut self,
        repairs: &[Repair],
        progress: &mut dyn FnMut(&Progress),
    ) -> Result<Report> {
        let mut report = Report::default();
        for (i, r) in repairs.iter().enumerate() {
            progress(&Progress {
                done: i,
                total: repairs.len(),
                bytes_done: report.bytes_copied,
                bytes_total: 0,
                current: &r.path,
            });
            let good = r.bad.other();
            let src = native(self.disk(good).root(), &r.path);
            let dst = native(self.disk(r.bad).root(), &r.path);
            let bad_id = self.disk(r.bad).id().to_string();
            let id = self
                .index_mut()
                .update(|tx| tx.journal_begin("repair", &format!("{r:?}")))?;
            match safe_copy(&src, &dst, Some(&r.hash), &mut |_| {}) {
                Ok(copied) => {
                    report.done += 1;
                    report.bytes_copied += copied.size;
                    let now = crate::now_rfc3339();
                    self.index_mut().update(|tx| {
                        tx.journal_end(id)?;
                        tx.set_verified(&r.key, &bad_id, &now)?;
                        tx.log("repaired", Some(&r.path), Some(&bad_id), None)
                    })?;
                }
                Err(e) => {
                    self.index_mut().update(|tx| tx.journal_end(id))?;
                    report.skipped.push(Skipped {
                        path: r.path.clone(),
                        reason: e.to_string(),
                    });
                }
            }
        }
        self.save()?;
        Ok(report)
    }

    fn flush_verified(
        &mut self,
        pending: &mut Vec<(String, Side)>,
        disk_ids: &[String; 2],
    ) -> Result<()> {
        if pending.is_empty() {
            return Ok(());
        }
        let now = crate::now_rfc3339();
        let batch = std::mem::take(pending);
        self.index_mut().update(|tx| {
            for (key, side) in &batch {
                tx.set_verified(key, &disk_ids[*side as usize], &now)?;
            }
            Ok(())
        })
    }
}

fn check_file(path: &std::path::Path, entry: &BaseEntry) -> Health {
    let Ok(meta) = fs::metadata(path) else {
        return Health::Missing;
    };
    let Ok(mtime) = mtime_ns(&meta) else {
        return Health::Changed;
    };
    if meta.len() != entry.size || !same_mtime(mtime, entry.mtime) {
        return Health::Changed;
    }
    match hash_file(path) {
        Ok(h) if Some(&h) == entry.hash.as_ref() => Health::Ok,
        Ok(_) => Health::Corrupt,
        Err(_) => Health::Missing,
    }
}

#[cfg(test)]
mod tests;
