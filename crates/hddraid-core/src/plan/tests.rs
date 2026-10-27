use std::cell::Cell;
use std::collections::HashMap;

use super::*;
use crate::scan::DirInfo;

const T: i64 = 1_700_000_000_000_000_000;
const LATER: i64 = T + 3_600_000_000_000;

/// Un disco finto: percorsi, dimensioni, date e contenuti (come hash).
#[derive(Default)]
struct FakeDisk {
    scan: Scan,
    hashes: HashMap<String, String>,
}

impl FakeDisk {
    fn file(mut self, path: &str, size: u64, mtime: i64, hash: &str) -> Self {
        self.scan.files.insert(
            paths::key(path),
            FileInfo {
                path: path.into(),
                size,
                mtime,
            },
        );
        self.hashes.insert(path.into(), hash.into());
        let mut p = paths::parent(path);
        while let Some(dir) = p {
            self.scan.dirs.insert(
                paths::key(dir),
                DirInfo {
                    path: dir.into(),
                    empty: false,
                },
            );
            p = paths::parent(dir);
        }
        self
    }

    fn empty_dir(mut self, path: &str) -> Self {
        self.scan.dirs.insert(
            paths::key(path),
            DirInfo {
                path: path.into(),
                empty: true,
            },
        );
        self
    }
}

fn disk() -> FakeDisk {
    FakeDisk::default()
}

fn base(entries: &[(&str, u64, i64, &str)]) -> Base {
    entries
        .iter()
        .map(|(p, size, mtime, h)| {
            (
                paths::key(p),
                BaseEntry {
                    path: (*p).into(),
                    kind: Kind::File,
                    size: *size,
                    mtime: *mtime,
                    hash: Some((*h).into()),
                },
            )
        })
        .collect()
}

fn base_dir(mut b: Base, path: &str) -> Base {
    b.insert(paths::key(path), dir_entry(path));
    b
}

fn run(base: &Base, a: &FakeDisk, b: &FakeDisk) -> (Plan, usize) {
    let calls = Cell::new(0);
    let mut hash = |side: Side, path: &str| {
        calls.set(calls.get() + 1);
        let disk = if side == Side::A { a } else { b };
        Ok(disk.hashes[path].clone())
    };
    let plan = plan(base, &a.scan, &b.scan, &mut hash).unwrap();
    (plan, calls.get())
}

fn info(path: &str, size: u64, mtime: i64) -> FileInfo {
    FileInfo {
        path: path.into(),
        size,
        mtime,
    }
}

#[test]
fn unchanged_disks_need_nothing() {
    let b = base(&[("a.jpg", 3, T, "h1")]);
    let d = || disk().file("a.jpg", 3, T, "h1");
    let (plan, calls) = run(&b, &d(), &d());
    assert!(plan.is_empty());
    assert_eq!(plan.actions, []);
    assert_eq!(calls, 0);
}

#[test]
fn mtime_within_tolerance_is_unchanged() {
    let b = base(&[("a.jpg", 3, T, "h1")]);
    let a = disk().file("a.jpg", 3, T + 1_000_000_000, "h1");
    let (plan, calls) = run(&b, &a, &disk().file("a.jpg", 3, T, "h1"));
    assert!(plan.actions.is_empty());
    assert_eq!(calls, 0);
}

#[test]
fn new_file_is_copied_without_hashing_first() {
    let a = disk().file("Foto/n.jpg", 5, T, "hn");
    let (plan, calls) = run(&Base::new(), &a, &disk());
    assert_eq!(
        plan.actions,
        [Action::Copy {
            from: Side::A,
            src: info("Foto/n.jpg", 5, T),
            dest_path: "Foto/n.jpg".into(),
            hash: None,
        }]
    );
    assert_eq!(plan.bytes_to(Side::B), 5);
    assert_eq!(calls, 0);
}

#[test]
fn modified_file_replaces_the_other() {
    let b = base(&[("Note.txt", 3, T, "h1")]);
    let a = disk().file("Note.txt", 4, LATER, "h2");
    let (plan, _) = run(&b, &a, &disk().file("Note.txt", 3, T, "h1"));
    assert_eq!(
        plan.actions,
        [Action::Copy {
            from: Side::A,
            src: info("Note.txt", 4, LATER),
            dest_path: "Note.txt".into(),
            hash: Some("h2".into()),
        }]
    );
    assert!(plan.questions.is_empty());
}

#[test]
fn touched_file_only_aligns_dates() {
    let b = base(&[("a.jpg", 3, T, "h1")]);
    let a = disk().file("a.jpg", 3, LATER, "h1");
    let (plan, _) = run(&b, &a, &disk().file("a.jpg", 3, T, "h1"));
    assert_eq!(
        plan.actions,
        [Action::SetMtime {
            side: Side::B,
            entry: BaseEntry {
                path: "a.jpg".into(),
                kind: Kind::File,
                size: 3,
                mtime: LATER,
                hash: Some("h1".into()),
            },
        }]
    );
}

#[test]
fn deletion_is_a_question() {
    let b = base(&[("x.jpg", 3, T, "h1")]);
    let (plan, _) = run(&b, &disk().file("x.jpg", 3, T, "h1"), &disk());
    assert!(plan.actions.is_empty());
    assert_eq!(plan.questions.len(), 1);
    let q = &plan.questions[0];
    assert_eq!(
        q.kind,
        QuestionKind::Deletion {
            deleted_on: Side::B
        }
    );
    assert!(q.b.is_none());
    assert_eq!(q.path(), "x.jpg");

    let empty = Scan::default();
    assert_eq!(
        resolve(q, Answer::DeleteEverywhere, "Disco B", &empty),
        [Action::Delete {
            side: Side::A,
            expect: info("x.jpg", 3, T),
            key: "x.jpg".into(),
        }]
    );
    assert_eq!(
        resolve(q, Answer::Restore, "Disco B", &empty),
        [Action::Copy {
            from: Side::A,
            src: info("x.jpg", 3, T),
            dest_path: "x.jpg".into(),
            hash: Some("h1".into()),
        }]
    );
}

#[test]
fn deleted_on_both_is_forgotten() {
    let b = base(&[("x.jpg", 3, T, "h1")]);
    let (plan, _) = run(&b, &disk(), &disk());
    assert_eq!(
        plan.actions,
        [Action::Forget {
            key: "x.jpg".into()
        }]
    );
    assert!(plan.is_empty());
}

#[test]
fn same_new_file_on_both_is_not_a_conflict() {
    let a = disk().file("r.pdf", 9, T, "same");
    let b = disk().file("r.pdf", 9, LATER, "same");
    let (plan, calls) = run(&Base::new(), &a, &b);
    assert!(plan.questions.is_empty());
    assert_eq!(calls, 2);
    assert!(matches!(
        &plan.actions[..],
        [Action::SetMtime { side: Side::B, entry }] if entry.mtime == T
    ));
}

#[test]
fn different_new_files_are_a_conflict() {
    let a = disk().file("Lista.txt", 5, T, "ha");
    let b = disk().file("Lista.txt", 5, T, "hb");
    let (plan, _) = run(&Base::new(), &a, &b);
    assert!(plan.actions.is_empty());
    let q = &plan.questions[0];
    assert_eq!(q.kind, QuestionKind::Conflict);
    assert_eq!(q.a.as_ref().unwrap().hash.as_deref(), Some("ha"));
    assert_eq!(q.b.as_ref().unwrap().hash.as_deref(), Some("hb"));

    let actions = resolve(q, Answer::KeepBoth, "Disco rosso", &b.scan);
    assert_eq!(
        actions,
        [
            Action::Rename {
                side: Side::B,
                from: "Lista.txt".into(),
                to: "Lista (Disco rosso).txt".into(),
            },
            Action::Copy {
                from: Side::B,
                src: info("Lista (Disco rosso).txt", 5, T),
                dest_path: "Lista (Disco rosso).txt".into(),
                hash: Some("hb".into()),
            },
            Action::Copy {
                from: Side::A,
                src: info("Lista.txt", 5, T),
                dest_path: "Lista.txt".into(),
                hash: Some("ha".into()),
            },
        ]
    );
    assert!(matches!(
        &resolve(q, Answer::Keep(Side::B), "B", &b.scan)[..],
        [Action::Copy { from: Side::B, .. }]
    ));
    assert!(resolve(q, Answer::Restore, "B", &b.scan).is_empty());
}

#[test]
fn alt_name_avoids_existing_files() {
    let b = disk()
        .file("Foto/a.jpg", 1, T, "x")
        .file("Foto/a (B).jpg", 1, T, "y");
    assert_eq!(alt_name("Foto/a.jpg", "B", &b.scan), "Foto/a (B 2).jpg");
    assert_eq!(alt_name("LEGGIMI", "B", &b.scan), "LEGGIMI (B)");
}

#[test]
fn both_modified_identically_is_not_a_conflict() {
    let b = base(&[("a.txt", 3, T, "old")]);
    let a = disk().file("a.txt", 4, LATER, "new");
    let bb = disk().file("a.txt", 4, LATER, "new");
    let (plan, _) = run(&b, &a, &bb);
    assert!(plan.questions.is_empty());
    assert!(matches!(&plan.actions[..], [Action::Record { .. }]));
}

#[test]
fn both_modified_differently_is_a_conflict() {
    let b = base(&[("a.txt", 3, T, "old")]);
    let a = disk().file("a.txt", 4, LATER, "na");
    let bb = disk().file("a.txt", 5, LATER, "nb");
    let (plan, _) = run(&b, &a, &bb);
    assert_eq!(plan.questions[0].kind, QuestionKind::Conflict);
}

#[test]
fn deleted_versus_modified_is_a_conflict() {
    let b = base(&[("a.txt", 3, T, "old")]);
    let bb = disk().file("a.txt", 4, LATER, "new");
    let (plan, _) = run(&b, &disk(), &bb);
    let q = &plan.questions[0];
    assert_eq!(q.kind, QuestionKind::Conflict);
    assert!(q.a.is_none());
    let empty = Scan::default();
    assert!(matches!(
        &resolve(q, Answer::Keep(Side::B), "B", &empty)[..],
        [Action::Copy { from: Side::B, .. }]
    ));
    assert!(matches!(
        &resolve(q, Answer::Keep(Side::A), "B", &empty)[..],
        [Action::Delete { side: Side::B, .. }]
    ));
}

#[test]
fn move_is_detected_by_hash() {
    let b = base(&[
        ("Foto/Montagna/1.jpg", 10, T, "h1"),
        ("Foto/Montagna/2.jpg", 20, T, "h2"),
    ]);
    let a =
        disk()
            .file("Foto/Dolomiti/1.jpg", 10, T, "h1")
            .file("Foto/Dolomiti/2.jpg", 20, T, "h2");
    let bb =
        disk()
            .file("Foto/Montagna/1.jpg", 10, T, "h1")
            .file("Foto/Montagna/2.jpg", 20, T, "h2");
    let (plan, _) = run(&b, &a, &bb);
    assert!(plan.questions.is_empty());
    let moves: Vec<_> = plan
        .actions
        .iter()
        .map(|a| match a {
            Action::Move { side, from, to, .. } => (*side, from.as_str(), to.as_str()),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        moves,
        [
            (Side::B, "Foto/Montagna/1.jpg", "Foto/Dolomiti/1.jpg"),
            (Side::B, "Foto/Montagna/2.jpg", "Foto/Dolomiti/2.jpg"),
        ]
    );
}

#[test]
fn ambiguous_move_falls_back_to_copy_and_questions() {
    let b = base(&[("x/1.jpg", 10, T, "h"), ("y/1.jpg", 10, T, "h")]);
    let a = disk().file("z/1.jpg", 10, T, "h");
    let bb = disk()
        .file("x/1.jpg", 10, T, "h")
        .file("y/1.jpg", 10, T, "h");
    let (plan, _) = run(&b, &a, &bb);
    assert!(matches!(
        &plan.actions[..],
        [Action::Copy { from: Side::A, .. }]
    ));
    assert_eq!(plan.questions.len(), 2);
}

#[test]
fn case_only_rename_is_propagated() {
    let b = base(&[("Lista.txt", 3, T, "h")]);
    let a = disk().file("LISTA.txt", 3, T, "h");
    let bb = disk().file("Lista.txt", 3, T, "h");
    let (plan, _) = run(&b, &a, &bb);
    assert!(matches!(
        &plan.actions[..],
        [Action::Move { side: Side::B, from, to, .. }] if from == "Lista.txt" && to == "LISTA.txt"
    ));
}

#[test]
fn adoption_merges_and_never_deletes() {
    let a = disk().file("1.jpg", 1, T, "h1").file("2.jpg", 2, T, "h2");
    let b = disk()
        .file("2.jpg", 2, LATER, "h2")
        .file("3.jpg", 3, T, "h3");
    let (plan, _) = run(&Base::new(), &a, &b);
    assert!(plan.questions.is_empty());
    assert!(
        !plan
            .actions
            .iter()
            .any(|a| matches!(a, Action::Delete { .. } | Action::RemoveDir { .. }))
    );
    let copies: Vec<_> = plan
        .actions
        .iter()
        .filter_map(|a| match a {
            Action::Copy { from, src, .. } => Some((*from, src.path.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(copies, [(Side::A, "1.jpg"), (Side::B, "3.jpg")]);
}

#[test]
fn empty_dirs() {
    // Nuova su A → creata su B.
    let (plan, _) = run(&Base::new(), &disk().empty_dir("2027"), &disk());
    assert_eq!(
        plan.actions,
        [Action::MakeDir {
            side: Side::B,
            path: "2027".into(),
        }]
    );

    // Vuota su entrambi e non nella base → entra nella base.
    let (plan, _) = run(
        &Base::new(),
        &disk().empty_dir("2027"),
        &disk().empty_dir("2027"),
    );
    assert!(matches!(&plan.actions[..], [Action::Record { entry }] if entry.kind == Kind::Dir));

    // Nella base, cancellata da B → domanda.
    let b = base_dir(Base::new(), "2027");
    let (plan, _) = run(&b, &disk().empty_dir("2027"), &disk());
    assert_eq!(
        plan.questions[0].kind,
        QuestionKind::Deletion {
            deleted_on: Side::B
        }
    );
    let q = &plan.questions[0];
    let empty = Scan::default();
    assert_eq!(
        resolve(q, Answer::Restore, "B", &empty),
        [Action::MakeDir {
            side: Side::B,
            path: "2027".into()
        }]
    );
    assert_eq!(
        resolve(q, Answer::DeleteEverywhere, "B", &empty),
        [Action::RemoveDir {
            side: Side::A,
            path: "2027".into()
        }]
    );

    // Nella base, ora con dei file su A → non si traccia più.
    let a = disk().file("2027/a.jpg", 1, T, "h");
    let (plan, _) = run(&b, &a, &disk().empty_dir("2027"));
    assert!(
        plan.actions
            .contains(&Action::Forget { key: "2027".into() })
    );

    // Sparita da entrambi → esce dalla base.
    let (plan, _) = run(&b, &disk(), &disk());
    assert_eq!(plan.actions, [Action::Forget { key: "2027".into() }]);
}
