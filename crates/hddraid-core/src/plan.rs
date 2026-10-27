//! Confronto a tre vie: ogni disco si confronta con la base (l'ultimo
//! allineamento), non con l'altro disco. Il risultato è un [`Plan`]: azioni
//! ovvie da eseguire e domande per l'utente (cancellazioni e conflitti).
//!
//! Le regole sono quelle di `doc/sincronizzazione.md`. Senza base (prima
//! adozione o indice perso) ogni file è *nuovo*: il piano unisce i dischi e
//! non cancella mai niente.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io;

use crate::archive::Side;
use crate::index::{Base, BaseEntry, Kind};
use crate::paths;
use crate::scan::{FileInfo, Scan, same_mtime};

/// Calcola l'hash del file `path` sul disco `side`.
pub type HashFn<'a> = dyn FnMut(Side, &str) -> io::Result<String> + 'a;

/// Un'azione da eseguire sui dischi (o solo sulla base).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Copia `src` dal disco `from` all'altro, in `dest_path`, sostituendo un
    /// eventuale file esistente. `hash`, se noto, è quello atteso della
    /// sorgente.
    Copy {
        from: Side,
        src: FileInfo,
        dest_path: String,
        hash: Option<String>,
    },
    /// Sposta o rinomina un file sul disco `side`; `entry` è la nuova voce
    /// della base, che prende il posto di `old_key`.
    Move {
        side: Side,
        from: String,
        to: String,
        old_key: String,
        entry: BaseEntry,
    },
    /// Cancella un file dal disco `side`, se è ancora com'era (`expect`).
    Delete {
        side: Side,
        expect: FileInfo,
        key: String,
    },
    /// Porta la data di modifica di un file su `side` a quella di `entry`:
    /// il contenuto è identico.
    SetMtime { side: Side, entry: BaseEntry },
    /// Crea una cartella vuota.
    MakeDir { side: Side, path: String },
    /// Cancella una cartella vuota.
    RemoveDir { side: Side, path: String },
    /// Rinomina un file per tenere entrambe le versioni di un conflitto.
    Rename {
        side: Side,
        from: String,
        to: String,
    },
    /// Aggiorna solo la base.
    Record { entry: BaseEntry },
    /// Toglie una voce dalla base.
    Forget { key: String },
}

impl Action {
    /// Byte da copiare e disco di destinazione, per le copie.
    pub fn copy_size(&self) -> Option<(Side, u64)> {
        match self {
            Action::Copy { from, src, .. } => Some((from.other(), src.size)),
            _ => None,
        }
    }
}

/// Una versione di un file (o cartella vuota) su un disco, per le domande.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub path: String,
    pub kind: Kind,
    pub size: u64,
    pub mtime: i64,
    pub hash: Option<String>,
}

impl Version {
    fn info(&self) -> FileInfo {
        FileInfo {
            path: self.path.clone(),
            size: self.size,
            mtime: self.mtime,
        }
    }
}

/// Tipo di domanda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionKind {
    /// Il file è stato cancellato da `deleted_on` e l'altro disco ha ancora
    /// la versione dell'ultimo allineamento.
    Deletion { deleted_on: Side },
    /// Le due versioni sono diverse (verificato con il checksum), oppure una
    /// è stata modificata e l'altra cancellata.
    Conflict,
}

/// Una domanda per l'utente. `a` e `b` sono le versioni sui due dischi
/// (`None` se il file manca su quel disco).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub key: String,
    pub kind: QuestionKind,
    pub a: Option<Version>,
    pub b: Option<Version>,
}

impl Question {
    pub fn version(&self, side: Side) -> Option<&Version> {
        match side {
            Side::A => self.a.as_ref(),
            Side::B => self.b.as_ref(),
        }
    }

    /// Percorso da mostrare.
    pub fn path(&self) -> &str {
        self.a
            .as_ref()
            .or(self.b.as_ref())
            .map_or("", |v| v.path.as_str())
    }
}

/// Risposta a una domanda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Cancella il file anche dove c'è ancora.
    DeleteEverywhere,
    /// Rimetti il file dove manca.
    Restore,
    /// Tieni la versione di questo disco (l'altra viene sostituita o
    /// cancellata).
    Keep(Side),
    /// Tieni entrambe le versioni: quella del disco B viene rinominata
    /// `nome (<nome del disco B>).ext`.
    KeepBoth,
}

/// Risultato del confronto.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Plan {
    pub actions: Vec<Action>,
    pub questions: Vec<Question>,
}

impl Plan {
    /// Niente da fare e niente da chiedere.
    pub fn is_empty(&self) -> bool {
        self.questions.is_empty()
            && self
                .actions
                .iter()
                .all(|a| matches!(a, Action::Record { .. } | Action::Forget { .. }))
    }

    /// Byte da copiare verso ciascun disco.
    pub fn bytes_to(&self, side: Side) -> u64 {
        self.actions
            .iter()
            .filter_map(Action::copy_size)
            .filter(|(s, _)| *s == side)
            .map(|(_, n)| n)
            .sum()
    }
}

/// Stato di un file su un disco rispetto alla base.
#[derive(Debug, Clone)]
enum State<'a> {
    /// Né nella base né sul disco.
    Absent,
    /// Sul disco ma non nella base.
    New(&'a FileInfo),
    /// Uguale alla base.
    Same(&'a FileInfo),
    /// Data diversa ma contenuto uguale alla base (verificato con l'hash).
    Touched(&'a FileInfo),
    /// Contenuto diverso dalla base; con il suo hash.
    Modified(&'a FileInfo, String),
    /// Nella base ma non sul disco.
    Deleted,
}

impl State<'_> {
    fn unchanged(&self) -> bool {
        matches!(self, State::Same(_) | State::Touched(_))
    }
}

/// Calcola il piano.
pub fn plan(base: &Base, a: &Scan, b: &Scan, hash: &mut HashFn) -> io::Result<Plan> {
    Planner {
        base,
        scans: [a, b],
        hash,
        cache: HashMap::new(),
        plan: Plan::default(),
    }
    .run()
}

struct Planner<'a, 'h> {
    base: &'a Base,
    scans: [&'a Scan; 2],
    hash: &'h mut HashFn<'h>,
    cache: HashMap<(Side, String), String>,
    plan: Plan,
}

impl<'a> Planner<'a, '_> {
    fn scan(&self, side: Side) -> &'a Scan {
        self.scans[side as usize]
    }

    fn hash_of(&mut self, side: Side, path: &str) -> io::Result<String> {
        let k = (side, path.to_string());
        if let Some(h) = self.cache.get(&k) {
            return Ok(h.clone());
        }
        let h = (self.hash)(side, path)?;
        self.cache.insert(k, h.clone());
        Ok(h)
    }

    fn base_file(&self, key: &str) -> Option<&'a BaseEntry> {
        self.base.get(key).filter(|e| e.kind == Kind::File)
    }

    fn state(&mut self, side: Side, key: &str) -> io::Result<State<'a>> {
        let cur = self.scan(side).files.get(key);
        Ok(match (self.base_file(key), cur) {
            (None, None) => State::Absent,
            (None, Some(c)) => State::New(c),
            (Some(_), None) => State::Deleted,
            (Some(b), Some(c)) => {
                if c.size == b.size && same_mtime(c.mtime, b.mtime) {
                    State::Same(c)
                } else {
                    let h = self.hash_of(side, &c.path)?;
                    if c.size == b.size && b.hash.as_deref() == Some(h.as_str()) {
                        State::Touched(c)
                    } else {
                        State::Modified(c, h)
                    }
                }
            }
        })
    }

    fn run(mut self) -> io::Result<Plan> {
        let keys: BTreeSet<&String> = self
            .base
            .iter()
            .filter(|(_, e)| e.kind == Kind::File)
            .map(|(k, _)| k)
            .chain(self.scans[0].files.keys())
            .chain(self.scans[1].files.keys())
            .collect();

        let mut states = BTreeMap::new();
        for key in keys {
            let sa = self.state(Side::A, key)?;
            let sb = self.state(Side::B, key)?;
            states.insert(key.clone(), [sa, sb]);
        }

        let moved = self.moves(&states)?;
        for (key, [sa, sb]) in &states {
            if !moved.contains(key) {
                self.file_rule(key, sa, sb)?;
            }
        }
        self.dirs();
        self.plan.questions.sort_by(|x, y| x.key.cmp(&y.key));
        Ok(self.plan)
    }

    /// Spostamenti e rinomine: un file cancellato su un disco e uno nuovo
    /// sullo stesso disco con lo stesso hash, mentre l'altro disco ha ancora
    /// il file vecchio. Solo abbinamenti univoci. Restituisce le chiavi usate.
    fn moves(&mut self, states: &BTreeMap<String, [State<'a>; 2]>) -> io::Result<BTreeSet<String>> {
        let mut used = BTreeSet::new();
        for x in Side::BOTH {
            let y = x.other();
            let (xi, yi) = (x as usize, y as usize);
            let deleted: Vec<(&String, &BaseEntry)> = states
                .iter()
                .filter(|(_, s)| matches!(s[xi], State::Deleted) && s[yi].unchanged())
                .filter_map(|(k, _)| self.base_file(k).map(|e| (k, e)))
                .filter(|(_, e)| e.hash.is_some())
                .collect();
            if deleted.is_empty() {
                continue;
            }
            let sizes: BTreeSet<u64> = deleted.iter().map(|(_, e)| e.size).collect();
            let mut by_hash: HashMap<String, MoveCandidates<'_, 'a>> = HashMap::new();
            for (k, e) in &deleted {
                by_hash
                    .entry(e.hash.clone().unwrap_or_default())
                    .or_default()
                    .deleted
                    .push(k);
            }
            for (k, s) in states {
                if let (State::New(f), State::Absent) = (&s[xi], &s[yi])
                    && sizes.contains(&f.size)
                {
                    let h = self.hash_of(x, &f.path)?;
                    if let Some(c) = by_hash.get_mut(&h) {
                        c.added.push((k, f));
                    }
                }
            }
            let mut found: Vec<_> = by_hash
                .into_iter()
                .filter(|(_, c)| c.deleted.len() == 1 && c.added.len() == 1)
                .map(|(h, c)| (c.deleted[0].clone(), c.added[0].0.clone(), c.added[0].1, h))
                .collect();
            found.sort_by(|x, y| x.0.cmp(&y.0));
            for (old_key, new_key, new_file, h) in found {
                let Some(from) = self.scan(y).files.get(&old_key) else {
                    continue;
                };
                self.plan.actions.push(Action::Move {
                    side: y,
                    from: from.path.clone(),
                    to: new_file.path.clone(),
                    old_key: old_key.clone(),
                    entry: BaseEntry {
                        path: new_file.path.clone(),
                        kind: Kind::File,
                        size: new_file.size,
                        mtime: new_file.mtime,
                        hash: Some(h),
                    },
                });
                used.insert(old_key);
                used.insert(new_key);
            }
        }
        Ok(used)
    }

    fn file_rule(&mut self, key: &str, sa: &State<'a>, sb: &State<'a>) -> io::Result<()> {
        use State::*;
        let base = self.base_file(key);
        match (sa, sb) {
            (Same(fa), Same(fb)) => self.case_rule(key, base, fa, fb),
            (Touched(f), Same(_) | Touched(_)) => self.align_mtime(Side::B, f, base),
            (Same(_), Touched(f)) => self.align_mtime(Side::A, f, base),
            (New(f), Absent) => self.copy(Side::A, f, None, None),
            (Absent, New(f)) => self.copy(Side::B, f, None, None),
            (New(fa), New(fb)) => {
                let ha = self.hash_of(Side::A, &fa.path)?;
                let hb = self.hash_of(Side::B, &fb.path)?;
                self.both_changed(key, fa, ha, fb, hb);
            }
            (Modified(f, h), Same(o) | Touched(o)) => {
                self.copy(Side::A, f, Some(h.clone()), Some(o))
            }
            (Same(o) | Touched(o), Modified(f, h)) => {
                self.copy(Side::B, f, Some(h.clone()), Some(o))
            }
            (Modified(fa, ha), Modified(fb, hb)) => {
                self.both_changed(key, fa, ha.clone(), fb, hb.clone())
            }
            (Deleted, Same(f) | Touched(f)) => self.ask_deletion(key, Side::A, f, base),
            (Same(f) | Touched(f), Deleted) => self.ask_deletion(key, Side::B, f, base),
            (Deleted, Deleted) => self.plan.actions.push(Action::Forget {
                key: key.to_string(),
            }),
            (Deleted, Modified(f, h)) => self.ask_conflict(key, None, Some(version(f, h))),
            (Modified(f, h), Deleted) => self.ask_conflict(key, Some(version(f, h)), None),
            // Combinazioni impossibili: New/Absent esistono solo senza base,
            // gli altri stati solo con la base.
            _ => {}
        }
        Ok(())
    }

    /// Stesso contenuto su entrambi: se un disco ha cambiato solo le
    /// maiuscole del nome, le cambia anche sull'altro.
    fn case_rule(&mut self, key: &str, base: Option<&BaseEntry>, fa: &FileInfo, fb: &FileInfo) {
        let Some(base) = base else { return };
        if fa.path == fb.path {
            if fa.path != base.path {
                self.plan.actions.push(Action::Record {
                    entry: BaseEntry {
                        path: fa.path.clone(),
                        ..base.clone()
                    },
                });
            }
            return;
        }
        let (renamed, other) = if fb.path == base.path {
            (fa, Side::B)
        } else {
            (fb, Side::A)
        };
        let from = if other == Side::B { fb } else { fa };
        self.plan.actions.push(Action::Move {
            side: other,
            from: from.path.clone(),
            to: renamed.path.clone(),
            old_key: key.to_string(),
            entry: BaseEntry {
                path: renamed.path.clone(),
                ..base.clone()
            },
        });
    }

    /// Contenuto uguale alla base ma data diversa su un disco: porta la data
    /// sull'altro disco.
    fn align_mtime(&mut self, side: Side, newer: &FileInfo, base: Option<&BaseEntry>) {
        let Some(base) = base else { return };
        let other = self.scan(side).files.get(&paths::key(&newer.path));
        let entry = BaseEntry {
            mtime: newer.mtime,
            ..base.clone()
        };
        match other {
            Some(o) if !same_mtime(o.mtime, newer.mtime) => {
                self.plan.actions.push(Action::SetMtime {
                    side,
                    entry: BaseEntry {
                        path: o.path.clone(),
                        ..entry
                    },
                })
            }
            _ => self.plan.actions.push(Action::Record { entry }),
        }
    }

    fn copy(&mut self, from: Side, src: &FileInfo, hash: Option<String>, dest: Option<&FileInfo>) {
        self.plan.actions.push(Action::Copy {
            from,
            src: src.clone(),
            dest_path: dest.map_or_else(|| src.path.clone(), |d| d.path.clone()),
            hash,
        });
    }

    /// Entrambi i dischi hanno una versione nuova o modificata: se il checksum
    /// è uguale sono identiche (si allineano solo le date), altrimenti è un
    /// conflitto.
    fn both_changed(&mut self, key: &str, fa: &FileInfo, ha: String, fb: &FileInfo, hb: String) {
        if ha != hb {
            self.ask_conflict(key, Some(version(fa, &ha)), Some(version(fb, &hb)));
            return;
        }
        let entry = BaseEntry {
            path: fa.path.clone(),
            kind: Kind::File,
            size: fa.size,
            mtime: fa.mtime,
            hash: Some(ha),
        };
        if same_mtime(fa.mtime, fb.mtime) {
            self.plan.actions.push(Action::Record { entry });
        } else {
            self.plan.actions.push(Action::SetMtime {
                side: Side::B,
                entry: BaseEntry {
                    path: fb.path.clone(),
                    ..entry
                },
            });
        }
    }

    fn ask_deletion(
        &mut self,
        key: &str,
        deleted_on: Side,
        kept: &FileInfo,
        base: Option<&BaseEntry>,
    ) {
        let v = Version {
            path: kept.path.clone(),
            kind: Kind::File,
            size: kept.size,
            mtime: kept.mtime,
            hash: base.and_then(|b| b.hash.clone()),
        };
        let (a, b) = match deleted_on {
            Side::A => (None, Some(v)),
            Side::B => (Some(v), None),
        };
        self.plan.questions.push(Question {
            key: key.to_string(),
            kind: QuestionKind::Deletion { deleted_on },
            a,
            b,
        });
    }

    fn ask_conflict(&mut self, key: &str, a: Option<Version>, b: Option<Version>) {
        self.plan.questions.push(Question {
            key: key.to_string(),
            kind: QuestionKind::Conflict,
            a,
            b,
        });
    }

    /// Cartelle vuote: stesse regole dei file. Una cartella con dei file non
    /// si traccia, esiste finché esistono i suoi file.
    fn dirs(&mut self) {
        let keys: BTreeSet<String> = self
            .base
            .iter()
            .filter(|(_, e)| e.kind == Kind::Dir)
            .map(|(k, _)| k.clone())
            .chain(
                Side::BOTH
                    .iter()
                    .flat_map(|s| self.scan(*s).dirs.iter())
                    .filter(|(_, d)| d.empty)
                    .map(|(k, _)| k.clone()),
            )
            .collect();
        for key in keys {
            let in_base = self.base.get(&key).is_some_and(|e| e.kind == Kind::Dir);
            let da = self.scans[0].dirs.get(&key);
            let db = self.scans[1].dirs.get(&key);
            match (da, db) {
                (Some(da), Some(db)) => {
                    if da.empty && db.empty {
                        if !in_base {
                            self.plan.actions.push(Action::Record {
                                entry: dir_entry(&da.path),
                            });
                        }
                    } else if in_base {
                        self.plan.actions.push(Action::Forget { key });
                    }
                }
                (Some(d), None) | (None, Some(d)) => {
                    let present = if da.is_some() { Side::A } else { Side::B };
                    if !d.empty {
                        if in_base {
                            self.plan.actions.push(Action::Forget { key });
                        }
                    } else if in_base {
                        let v = Some(Version {
                            path: d.path.clone(),
                            kind: Kind::Dir,
                            size: 0,
                            mtime: 0,
                            hash: None,
                        });
                        let (a, b) = match present {
                            Side::A => (v, None),
                            Side::B => (None, v),
                        };
                        self.plan.questions.push(Question {
                            key,
                            kind: QuestionKind::Deletion {
                                deleted_on: present.other(),
                            },
                            a,
                            b,
                        });
                    } else {
                        self.plan.actions.push(Action::MakeDir {
                            side: present.other(),
                            path: d.path.clone(),
                        });
                    }
                }
                (None, None) => {
                    if in_base {
                        self.plan.actions.push(Action::Forget { key });
                    }
                }
            }
        }
    }
}

/// File cancellati e nuovi con lo stesso hash, sullo stesso disco.
#[derive(Default)]
struct MoveCandidates<'k, 'a> {
    deleted: Vec<&'k String>,
    added: Vec<(&'k String, &'a FileInfo)>,
}

fn version(f: &FileInfo, hash: &str) -> Version {
    Version {
        path: f.path.clone(),
        kind: Kind::File,
        size: f.size,
        mtime: f.mtime,
        hash: Some(hash.to_string()),
    }
}

fn dir_entry(path: &str) -> BaseEntry {
    BaseEntry {
        path: path.to_string(),
        kind: Kind::Dir,
        size: 0,
        mtime: 0,
        hash: None,
    }
}

/// Azioni che corrispondono alla risposta a una domanda. `label_b` è il nome
/// del disco B, usato per "tienile entrambe". Una risposta che non ha senso
/// per la domanda non produce azioni.
pub fn resolve(question: &Question, answer: Answer, label_b: &str, b: &Scan) -> Vec<Action> {
    match (&question.a, &question.b) {
        (Some(va), Some(vb)) => match answer {
            Answer::Keep(side) => {
                let (src, dst) = match side {
                    Side::A => (va, vb),
                    Side::B => (vb, va),
                };
                vec![Action::Copy {
                    from: side,
                    src: src.info(),
                    dest_path: dst.path.clone(),
                    hash: src.hash.clone(),
                }]
            }
            Answer::KeepBoth => {
                let alt = alt_name(&vb.path, label_b, b);
                vec![
                    Action::Rename {
                        side: Side::B,
                        from: vb.path.clone(),
                        to: alt.clone(),
                    },
                    Action::Copy {
                        from: Side::B,
                        src: FileInfo {
                            path: alt.clone(),
                            ..vb.info()
                        },
                        dest_path: alt,
                        hash: vb.hash.clone(),
                    },
                    Action::Copy {
                        from: Side::A,
                        src: va.info(),
                        dest_path: va.path.clone(),
                        hash: va.hash.clone(),
                    },
                ]
            }
            Answer::DeleteEverywhere | Answer::Restore => vec![],
        },
        (Some(v), None) | (None, Some(v)) => {
            let present = if question.a.is_some() {
                Side::A
            } else {
                Side::B
            };
            let keep = match answer {
                Answer::Restore | Answer::KeepBoth => true,
                Answer::DeleteEverywhere => false,
                Answer::Keep(side) => side == present,
            };
            match (v.kind, keep) {
                (Kind::File, true) => vec![Action::Copy {
                    from: present,
                    src: v.info(),
                    dest_path: v.path.clone(),
                    hash: v.hash.clone(),
                }],
                (Kind::File, false) => vec![Action::Delete {
                    side: present,
                    expect: v.info(),
                    key: question.key.clone(),
                }],
                (Kind::Dir, true) => vec![Action::MakeDir {
                    side: present.other(),
                    path: v.path.clone(),
                }],
                (Kind::Dir, false) => vec![Action::RemoveDir {
                    side: present,
                    path: v.path.clone(),
                }],
            }
        }
        (None, None) => vec![],
    }
}

/// `nome (etichetta).ext`, con un numero se esiste già sul disco B.
fn alt_name(path: &str, label: &str, b: &Scan) -> String {
    let name = paths::file_name(path);
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s, format!(".{e}")),
        _ => (name, String::new()),
    };
    (1..)
        .map(|n| {
            let suffix = if n == 1 {
                format!("({label})")
            } else {
                format!("({label} {n})")
            };
            paths::with_file_name(path, &format!("{stem} {suffix}{ext}"))
        })
        .find(|p| !b.files.contains_key(&paths::key(p)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
