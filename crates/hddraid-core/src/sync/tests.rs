//! Test end-to-end sui dischi simulati di `hddraid-devtools`.

use std::collections::BTreeMap;
use std::path::Path;

use hddraid_devtools::{DevDisks, Scenario};

use super::*;
use crate::disk::DATA_DIR;
use crate::plan::QuestionKind;

struct Env {
    _dir: tempfile::TempDir,
    disks: DevDisks,
    archive: Archive,
}

/// Due dischi simulati uguali, preparati e allineati una prima volta.
fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let disks = DevDisks::in_dir(dir.path());
    disks.setup().unwrap();
    let mut archive = Archive::create(
        &disks.a,
        &disks.b,
        ["Disco A", "Disco B"],
        &dir.path().join("work"),
    )
    .unwrap();
    let plan = archive.plan().unwrap();
    assert!(plan.questions.is_empty());
    assert_eq!(plan.bytes_to(Side::A) + plan.bytes_to(Side::B), 0);
    archive.apply(&plan.actions, &mut |_| {}).unwrap();
    Env {
        _dir: dir,
        disks,
        archive,
    }
}

impl Env {
    /// Piano, risposte con `answer`, esecuzione.
    fn sync(&mut self, answer: impl Fn(&Question) -> Option<Answer>) -> (Plan, Report) {
        let plan = self.archive.plan().unwrap();
        let mut actions = plan.actions.clone();
        for q in &plan.questions {
            if let Some(a) = answer(q) {
                actions.extend(self.archive.resolve(q, a));
            }
        }
        let report = self.archive.apply(&actions, &mut |_| {}).unwrap();
        (plan, report)
    }

    fn assert_aligned(&self) {
        assert_eq!(snapshot(&self.disks.a), snapshot(&self.disks.b));
        let plan = self.archive.plan().unwrap();
        assert!(plan.is_empty(), "piano non vuoto: {plan:#?}");
    }
}

/// Contenuto di un disco (escluse la cartella dell'app e i file ignorati):
/// percorso → hash del contenuto, `None` per le cartelle.
fn snapshot(root: &Path) -> BTreeMap<String, Option<String>> {
    let mut out = BTreeMap::new();
    let scan = scan::scan(root).unwrap();
    for f in scan.files.values() {
        out.insert(
            f.path.clone(),
            Some(hash_file(&native(root, &f.path)).unwrap()),
        );
    }
    for d in scan.dirs.values() {
        out.insert(d.path.clone(), None);
    }
    out
}

fn never(_: &Question) -> Option<Answer> {
    None
}

#[test]
fn identical_disks_are_adopted_without_copies() {
    let e = env();
    assert!(e.archive.index().base().unwrap().len() > 20);
    assert!(e.archive.index().meta("last_sync").unwrap().is_some());
    e.assert_aligned();
}

#[test]
fn new_files_are_copied() {
    let mut e = env();
    e.disks.apply(Scenario::New).unwrap();
    let (plan, report) = e.sync(never);
    assert_eq!(plan.questions.len(), 0);
    let copies = plan
        .actions
        .iter()
        .filter(|a| matches!(a, Action::Copy { .. }));
    assert_eq!(copies.count(), 5);
    assert_eq!(report.done, plan.actions.len());
    assert!(report.bytes_copied > 0);
    assert!(
        e.disks
            .b
            .join("Foto/2026/Natale 2026/IMG_0003.jpg")
            .exists()
    );
    e.assert_aligned();
}

#[test]
fn modified_file_is_copied() {
    let mut e = env();
    e.disks.apply(Scenario::Modify).unwrap();
    e.sync(never);
    let note = fs::read_to_string(e.disks.b.join("Documenti/Note.txt")).unwrap();
    assert!(note.contains("disco A"));
    e.assert_aligned();
}

#[test]
fn deletion_confirmed() {
    let mut e = env();
    e.disks.apply(Scenario::Delete).unwrap();
    let (plan, _) = e.sync(|_| Some(Answer::DeleteEverywhere));
    assert_eq!(
        plan.questions[0].kind,
        QuestionKind::Deletion {
            deleted_on: Side::B
        }
    );
    assert!(!e.disks.a.join("Foto/2024/Mare 2024/IMG_0002.jpg").exists());
    e.assert_aligned();
}

#[test]
fn deletion_restored() {
    let mut e = env();
    e.disks.apply(Scenario::Delete).unwrap();
    e.sync(|_| Some(Answer::Restore));
    assert!(e.disks.b.join("Foto/2024/Mare 2024/IMG_0002.jpg").exists());
    e.assert_aligned();
}

#[test]
fn unanswered_deletion_is_asked_again() {
    let mut e = env();
    e.disks.apply(Scenario::Delete).unwrap();
    e.sync(never);
    let plan = e.archive.plan().unwrap();
    assert_eq!(plan.questions.len(), 1);
}

#[test]
fn moved_folder_is_moved_not_copied() {
    let mut e = env();
    e.disks.apply(Scenario::Move).unwrap();
    let (plan, report) = e.sync(never);
    assert!(plan.questions.is_empty());
    assert_eq!(report.bytes_copied, 0);
    assert!(
        e.disks
            .b
            .join("Foto/2023/Dolomiti 2023/IMG_0001.jpg")
            .exists()
    );
    assert!(!e.disks.b.join("Foto/2023/Montagna").exists());
    e.assert_aligned();
}

#[test]
fn conflict_keep_both() {
    let mut e = env();
    e.disks.apply(Scenario::Conflict).unwrap();
    let (plan, _) = e.sync(|_| Some(Answer::KeepBoth));
    assert_eq!(plan.questions[0].kind, QuestionKind::Conflict);
    for disk in [&e.disks.a, &e.disks.b] {
        let a = fs::read_to_string(disk.join("Documenti/Lista.txt")).unwrap();
        let b = fs::read_to_string(disk.join("Documenti/Lista (Disco B).txt")).unwrap();
        assert!(a.contains("disco A"));
        assert!(b.contains("disco B"));
    }
    e.assert_aligned();
}

#[test]
fn conflict_keep_one() {
    let mut e = env();
    e.disks.apply(Scenario::Conflict).unwrap();
    e.sync(|_| Some(Answer::Keep(Side::B)));
    let a = fs::read_to_string(e.disks.a.join("Documenti/Lista.txt")).unwrap();
    assert!(a.contains("disco B"));
    e.assert_aligned();
}

#[test]
fn same_content_is_not_a_conflict() {
    let mut e = env();
    e.disks.apply(Scenario::SameContent).unwrap();
    let (plan, report) = e.sync(never);
    assert!(plan.questions.is_empty());
    assert_eq!(report.bytes_copied, 0);
    let ma = fs::metadata(e.disks.a.join("Documenti/Ricevuta.pdf")).unwrap();
    let mb = fs::metadata(e.disks.b.join("Documenti/Ricevuta.pdf")).unwrap();
    assert!(same_mtime(mtime_ns(&ma).unwrap(), mtime_ns(&mb).unwrap()));
    e.assert_aligned();
}

#[test]
fn corruption_is_not_seen_by_sync() {
    let e = env();
    e.disks.apply(Scenario::Corrupt).unwrap();
    assert!(e.archive.plan().unwrap().is_empty());
}

#[test]
fn empty_dir_and_case_rename() {
    let mut e = env();
    e.disks.apply(Scenario::EmptyDir).unwrap();
    e.disks.apply(Scenario::Case).unwrap();
    e.sync(never);
    assert!(e.disks.b.join("Foto/2027").is_dir());
    let names: Vec<_> = fs::read_dir(e.disks.b.join("Documenti"))
        .unwrap()
        .map(|d| d.unwrap().file_name().into_string().unwrap())
        .collect();
    assert!(names.contains(&"LISTA.txt".to_string()), "{names:?}");
    e.assert_aligned();
}

#[test]
fn system_files_are_ignored() {
    let e = env();
    e.disks.apply(Scenario::System).unwrap();
    assert!(e.archive.plan().unwrap().is_empty());
}

#[test]
fn all_scenarios_together() {
    let mut e = env();
    // Tutti tranne `Corrupt`: il bit rot lo trova il controllo completo.
    for (name, scenario) in Scenario::NAMES {
        if !matches!(name, "all" | "corrupt") {
            e.disks.apply(scenario).unwrap();
        }
    }
    e.sync(|q| {
        Some(match q.kind {
            QuestionKind::Deletion { .. } => Answer::Restore,
            QuestionKind::Conflict => Answer::KeepBoth,
        })
    });
    e.assert_aligned();
}

#[test]
fn adoption_merges_different_disks() {
    let dir = tempfile::tempdir().unwrap();
    let disks = DevDisks::in_dir(dir.path());
    disks.setup().unwrap();
    fs::write(disks.a.join("solo-a.txt"), b"a").unwrap();
    fs::write(disks.b.join("solo-b.txt"), b"b").unwrap();
    fs::write(disks.b.join("Documenti/Note.txt"), b"diverso").unwrap();
    let mut archive =
        Archive::create(&disks.a, &disks.b, ["A", "B"], &dir.path().join("w")).unwrap();
    let plan = archive.plan().unwrap();
    assert_eq!(plan.questions.len(), 1);
    assert_eq!(plan.questions[0].kind, QuestionKind::Conflict);
    archive.apply(&plan.actions, &mut |_| {}).unwrap();
    assert!(disks.a.join("solo-b.txt").exists());
    assert!(disks.b.join("solo-a.txt").exists());
}

#[test]
fn file_changed_after_plan_is_skipped() {
    let mut e = env();
    e.disks.apply(Scenario::New).unwrap();
    let plan = e.archive.plan().unwrap();
    let changed = e.disks.a.join("Foto/2026/Natale 2026/IMG_0001.jpg");
    fs::write(&changed, b"cambiato durante l'allineamento").unwrap();
    let report = e.archive.apply(&plan.actions, &mut |_| {}).unwrap();
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.done, plan.actions.len() - 1);
    assert!(e.archive.index().journal().unwrap().is_empty());
    e.sync(never);
    e.assert_aligned();
}

#[test]
fn progress_reaches_the_end() {
    let mut e = env();
    e.disks.apply(Scenario::New).unwrap();
    let plan = e.archive.plan().unwrap();
    let mut last = None;
    e.archive
        .apply(&plan.actions, &mut |p| {
            last = Some((p.done, p.total, p.bytes_done, p.bytes_total))
        })
        .unwrap();
    let (done, total, bytes_done, bytes_total) = last.unwrap();
    assert_eq!(done, total);
    assert_eq!(bytes_done, bytes_total);
}

#[test]
fn interrupted_operation_is_reported_and_healed() {
    let mut e = env();
    e.disks.apply(Scenario::New).unwrap();
    // Simula un'interruzione: l'azione è nel journal e il file è già stato
    // copiato su B, ma la base non è stata aggiornata.
    e.archive
        .index_mut()
        .update(|tx| tx.journal_begin("copy", "IMG_0001"))
        .unwrap();
    e.archive.save().unwrap();
    let src = e.disks.a.join("Foto/2026/Natale 2026/IMG_0001.jpg");
    let dst = e.disks.b.join("Foto/2026/Natale 2026/IMG_0001.jpg");
    safe_copy(&src, &dst, None, &mut |_| {}).unwrap();

    // Riapre la coppia come un altro computer: la copia di lavoro di
    // `e.archive` è ancora aperta, e Windows non permette di sostituirla.
    let work = e._dir.path().join("work-2");
    let mut archive = Archive::open(&e.disks.a, &e.disks.b, &work).unwrap();
    assert_eq!(archive.open_info().interrupted.len(), 1);
    let plan = archive.plan().unwrap();
    assert!(plan.questions.is_empty());
    assert!(plan.bytes_to(Side::B) > 0);
    archive.apply(&plan.actions, &mut |_| {}).unwrap();
    archive.index_mut().update(|tx| tx.journal_clear()).unwrap();
    e.archive = archive;
    e.assert_aligned();
}

#[test]
fn data_dir_is_never_synced() {
    let mut e = env();
    fs::write(e.disks.a.join(DATA_DIR).join("extra"), b"x").unwrap();
    assert!(e.archive.plan().unwrap().is_empty());
    e.sync(never);
    assert!(!e.disks.b.join(DATA_DIR).join("extra").exists());
}
