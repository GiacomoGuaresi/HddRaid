use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use hddraid_devtools::{DevDisks, Scenario};

use super::*;

struct Env {
    _dir: tempfile::TempDir,
    disks: DevDisks,
    archive: Archive,
}

/// Due dischi simulati uguali, preparati e allineati.
fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let disks = DevDisks::in_dir(dir.path());
    disks.setup().unwrap();
    let mut archive =
        Archive::create(&disks.a, &disks.b, ["A", "B"], &dir.path().join("w")).unwrap();
    let plan = archive.plan().unwrap();
    archive.apply(&plan.actions, &mut |_| {}).unwrap();
    Env {
        _dir: dir,
        disks,
        archive,
    }
}

fn check(archive: &mut Archive) -> (CheckReport, usize) {
    let mut total = 0;
    let report = archive
        .full_check(&mut |p| total = p.total, &AtomicBool::new(false))
        .unwrap();
    (report, total)
}

/// Cambia un byte lasciando dimensione e data uguali.
fn rot(path: &Path) {
    let mtime = fs::metadata(path).unwrap().modified().unwrap();
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mut b = [0u8];
    f.seek(SeekFrom::Start(10)).unwrap();
    f.read_exact(&mut b).unwrap();
    f.seek(SeekFrom::Start(10)).unwrap();
    f.write_all(&[b[0] ^ 0x55]).unwrap();
    f.set_modified(mtime).unwrap();
}

const PHOTO: &str = "Foto/2025/Compleanno/IMG_0001.jpg";

#[test]
fn healthy_disks() {
    let mut e = env();
    assert!(e.archive.check_due(Duration::from_secs(3600)).unwrap());
    let files = e
        .archive
        .index()
        .base()
        .unwrap()
        .values()
        .filter(|e| e.kind == Kind::File)
        .count();
    let (report, total) = check(&mut e.archive);
    assert!(report.completed);
    assert_eq!(total, files * 2);
    assert_eq!(report.ok, files);
    assert!(report.repairs.is_empty() && report.lost.is_empty() && report.changed.is_empty());
    assert!(!e.archive.check_due(Duration::from_secs(3600)).unwrap());
    assert!(e.archive.check_due(Duration::ZERO).unwrap());
}

#[test]
fn bit_rot_is_found_and_repaired() {
    let mut e = env();
    e.disks.apply(Scenario::Corrupt).unwrap();
    let (report, _) = check(&mut e.archive);
    assert_eq!(report.repairs.len(), 1);
    let r = &report.repairs[0];
    assert_eq!(r.path, PHOTO);
    assert_eq!(r.bad, Side::B);

    let done = e.archive.repair(&report.repairs, &mut |_| {}).unwrap();
    assert_eq!(done.done, 1);
    assert_eq!(
        fs::read(e.disks.a.join(PHOTO)).unwrap(),
        fs::read(e.disks.b.join(PHOTO)).unwrap()
    );
    let (report, _) = check(&mut e.archive);
    assert!(report.repairs.is_empty());
    assert!(e.archive.plan().unwrap().is_empty());
}

#[test]
fn rotten_on_both_is_lost() {
    let mut e = env();
    rot(&e.disks.a.join(PHOTO));
    rot(&e.disks.b.join(PHOTO));
    let (report, _) = check(&mut e.archive);
    assert!(report.repairs.is_empty());
    assert_eq!(report.lost, [PHOTO]);
}

#[test]
fn modified_file_is_not_corruption() {
    let mut e = env();
    e.disks.apply(Scenario::Modify).unwrap();
    let (report, _) = check(&mut e.archive);
    assert!(report.repairs.is_empty());
    assert_eq!(report.changed, ["Documenti/Note.txt"]);
}

#[test]
fn interrupted_check_resumes() {
    let mut e = env();
    let cancel = AtomicBool::new(false);
    let mut first_total = 0;
    let report = e
        .archive
        .full_check(
            &mut |p| {
                first_total = p.total;
                if p.checked >= 6 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            &cancel,
        )
        .unwrap();
    assert!(!report.completed);
    assert!(
        e.archive
            .index()
            .meta("full_check_started")
            .unwrap()
            .is_some()
    );
    assert!(e.archive.check_due(Duration::from_secs(3600)).unwrap());

    let (report, total) = check(&mut e.archive);
    assert!(report.completed);
    assert!(total < first_total, "{total} < {first_total}");
    assert!(total > 0);
    let files = e
        .archive
        .index()
        .base()
        .unwrap()
        .values()
        .filter(|e| e.kind == Kind::File)
        .count();
    assert_eq!(report.ok, files);
    assert!(
        e.archive
            .index()
            .meta("full_check_started")
            .unwrap()
            .is_none()
    );
}
