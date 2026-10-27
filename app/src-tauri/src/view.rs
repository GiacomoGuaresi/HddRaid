//! Lo stato mostrato dall'interfaccia, serializzato come in
//! `src/lib/types.ts`.

use std::collections::BTreeMap;

use hddraid_core::Side;
use hddraid_core::check::Repair;
use hddraid_core::index::{Base, Kind};
use hddraid_core::paths::key;
use hddraid_core::plan::{Action, Plan, Question, QuestionKind, Version};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiskView {
    pub side: Side,
    pub label: String,
    pub root: String,
    pub total: u64,
    pub free: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineKind {
    New,
    Changed,
    Moved,
    Folders,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SummaryLine {
    pub kind: LineKind,
    pub from: Side,
    pub count: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FileLine {
    pub kind: LineKind,
    pub from: Side,
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VersionView {
    pub size: u64,
    pub modified: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuestionView {
    pub key: String,
    pub path: String,
    pub kind: &'static str,
    pub deleted_on: Option<Side>,
    pub is_dir: bool,
    pub a: Option<VersionView>,
    pub b: Option<VersionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Summary {
    pub lines: Vec<SummaryLine>,
    pub files: Vec<FileLine>,
    pub questions: Vec<QuestionView>,
    pub missing_space: BTreeMap<Side, u64>,
}

impl Summary {
    /// File e cartelle da allineare, senza contare le domande.
    pub fn changes(&self) -> usize {
        self.lines.iter().map(|l| l.count).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.questions.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProgressView {
    pub done: usize,
    pub total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub current: String,
    pub eta: Option<u64>,
}

impl ProgressView {
    pub fn percent(&self) -> u32 {
        let (done, total) = if self.bytes_total > 0 {
            (self.bytes_done, self.bytes_total)
        } else {
            (self.done as u64, self.total as u64)
        };
        if total == 0 {
            0
        } else {
            ((done * 100) / total).min(100) as u32
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Busy {
    Comparing,
    Syncing { progress: ProgressView },
    Checking { progress: ProgressView },
    Repairing { progress: ProgressView },
    Ejecting,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SkippedView {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RepairView {
    pub path: String,
    pub bad: Side,
}

impl From<&Repair> for RepairView {
    fn from(r: &Repair) -> Self {
        RepairView {
            path: r.path.clone(),
            bad: r.bad,
        }
    }
}

/// Perché un'operazione non è riuscita, per un messaggio comprensibile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Un disco è stato scollegato.
    Disconnected,
    /// Manca spazio su un disco.
    NoSpace,
    /// Non si riesce a scrivere su un disco.
    ReadOnly,
    /// Il disco non si espelle, di solito perché un file è aperto.
    Eject,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Outcome {
    Synced {
        copied: usize,
        bytes: u64,
        deleted: usize,
        skipped: Vec<SkippedView>,
    },
    Checked {
        ok: usize,
        repairs: Vec<RepairView>,
        lost: Vec<String>,
        completed: bool,
    },
    Repaired {
        done: usize,
        skipped: Vec<SkippedView>,
    },
    Ejected,
    Error {
        code: ErrorCode,
        disk: Option<Side>,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PairView {
    pub labels: BTreeMap<Side, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct State {
    pub pair: Option<PairView>,
    pub disks: BTreeMap<Side, Option<DiskView>>,
    pub foreign: Vec<String>,
    pub busy: Option<Busy>,
    pub summary: Option<Summary>,
    pub outcome: Option<Outcome>,
    pub last_sync: Option<String>,
    pub last_check: Option<String>,
    pub check_due: bool,
    /// Durata stimata del controllo completo, in secondi.
    pub check_estimate: Option<u64>,
}

impl Default for State {
    fn default() -> Self {
        State {
            pair: None,
            disks: Side::BOTH.map(|s| (s, None)).into_iter().collect(),
            foreign: Vec::new(),
            busy: None,
            summary: None,
            outcome: None,
            last_sync: None,
            last_check: None,
            check_due: false,
            check_estimate: None,
        }
    }
}

/// La risposta a una domanda, come la manda l'interfaccia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerView {
    Delete,
    Restore,
    KeepA,
    KeepB,
    Both,
    Later,
}

impl AnswerView {
    pub fn to_core(self) -> Option<hddraid_core::plan::Answer> {
        use hddraid_core::plan::Answer;
        match self {
            AnswerView::Delete => Some(Answer::DeleteEverywhere),
            AnswerView::Restore => Some(Answer::Restore),
            AnswerView::KeepA => Some(Answer::Keep(Side::A)),
            AnswerView::KeepB => Some(Answer::Keep(Side::B)),
            AnswerView::Both => Some(Answer::KeepBoth),
            AnswerView::Later => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryEvent {
    pub at: String,
    pub kind: String,
    pub path: Option<String>,
    pub disk: Option<String>,
    pub details: Option<String>,
}

/// Riepilogo del piano a parole semplici: cosa è cambiato e dove.
pub fn summary(plan: &Plan, base: &Base, free: [u64; 2]) -> Summary {
    let mut files = Vec::new();
    for action in &plan.actions {
        let line = match action {
            Action::Copy {
                from,
                src,
                dest_path,
                ..
            } => {
                let kind = if base.contains_key(&key(dest_path)) {
                    LineKind::Changed
                } else {
                    LineKind::New
                };
                Some((kind, *from, src.path.clone(), src.size))
            }
            Action::Move {
                side, to, entry, ..
            } => Some((LineKind::Moved, side.other(), to.clone(), entry.size)),
            Action::Rename { side, to, .. } => Some((LineKind::Moved, side.other(), to.clone(), 0)),
            Action::MakeDir { side, path } => {
                Some((LineKind::Folders, side.other(), path.clone(), 0))
            }
            _ => None,
        };
        if let Some((kind, from, path, size)) = line {
            files.push(FileLine {
                kind,
                from,
                path,
                size,
            });
        }
    }
    files.sort_by(|x, y| (x.kind, x.from, &x.path).cmp(&(y.kind, y.from, &y.path)));

    let mut lines: Vec<SummaryLine> = Vec::new();
    for f in &files {
        match lines.last_mut() {
            Some(l) if l.kind == f.kind && l.from == f.from => {
                l.count += 1;
                l.bytes += f.size;
            }
            _ => lines.push(SummaryLine {
                kind: f.kind,
                from: f.from,
                count: 1,
                bytes: f.size,
            }),
        }
    }

    let missing_space = Side::BOTH
        .map(|s| (s, plan.bytes_to(s).saturating_sub(free[s as usize])))
        .into_iter()
        .collect();

    Summary {
        lines,
        files,
        questions: plan.questions.iter().map(question).collect(),
        missing_space,
    }
}

fn question(q: &Question) -> QuestionView {
    let (kind, deleted_on) = match q.kind {
        QuestionKind::Deletion { deleted_on } => ("deletion", Some(deleted_on)),
        QuestionKind::Conflict => ("conflict", None),
    };
    QuestionView {
        key: q.key.clone(),
        path: q.path().to_string(),
        kind,
        deleted_on,
        is_dir: [&q.a, &q.b]
            .into_iter()
            .flatten()
            .any(|v| v.kind == Kind::Dir),
        a: q.a.as_ref().map(version),
        b: q.b.as_ref().map(version),
    }
}

fn version(v: &Version) -> VersionView {
    VersionView {
        size: v.size,
        modified: chrono::DateTime::from_timestamp_nanos(v.mtime).to_rfc3339(),
        preview: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hddraid_core::index::BaseEntry;
    use hddraid_core::scan::FileInfo;

    fn copy(from: Side, path: &str, size: u64) -> Action {
        Action::Copy {
            from,
            src: FileInfo {
                path: path.into(),
                size,
                mtime: 0,
            },
            dest_path: path.into(),
            hash: None,
        }
    }

    #[test]
    fn groups_actions_by_kind_and_disk() {
        let mut base = Base::new();
        base.insert(
            "doc/note.txt".into(),
            BaseEntry {
                path: "Doc/Note.txt".into(),
                kind: Kind::File,
                size: 1,
                mtime: 0,
                hash: None,
            },
        );
        let plan = Plan {
            actions: vec![
                copy(Side::A, "Foto/1.jpg", 10),
                copy(Side::A, "Foto/2.jpg", 20),
                copy(Side::B, "Doc/Note.txt", 5),
                Action::MakeDir {
                    side: Side::B,
                    path: "Vuota".into(),
                },
            ],
            questions: vec![],
        };
        let s = summary(&plan, &base, [100, 25]);
        assert_eq!(
            s.lines,
            [
                SummaryLine {
                    kind: LineKind::New,
                    from: Side::A,
                    count: 2,
                    bytes: 30
                },
                SummaryLine {
                    kind: LineKind::Changed,
                    from: Side::B,
                    count: 1,
                    bytes: 5
                },
                SummaryLine {
                    kind: LineKind::Folders,
                    from: Side::A,
                    count: 1,
                    bytes: 0
                },
            ]
        );
        assert_eq!(s.changes(), 4);
        assert_eq!(s.missing_space[&Side::B], 5);
        assert_eq!(s.missing_space[&Side::A], 0);
    }
}
