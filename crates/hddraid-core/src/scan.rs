//! Scansione di un disco: percorso, dimensione e data di modifica di ogni
//! file, più le cartelle. Non legge il contenuto.

use std::collections::BTreeMap;
use std::fs::{self, Metadata};
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths;

/// Tolleranza nel confronto delle date: exFAT salva le date con risoluzione
/// di 10 ms e alcuni sistemi le arrotondano.
pub const MTIME_TOLERANCE_NS: i64 = 2_000_000_000;

/// Un file trovato sul disco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    /// Percorso relativo, con `/`, in NFC.
    pub path: String,
    pub size: u64,
    /// Data di modifica, in nanosecondi dal 1970 (UTC).
    pub mtime: i64,
}

/// Una cartella trovata sul disco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirInfo {
    pub path: String,
    /// Nessun file e nessuna sottocartella (esclusi quelli ignorati).
    pub empty: bool,
}

/// Risultato della scansione, per chiave di percorso (vedi [`paths::key`]).
#[derive(Debug, Default)]
pub struct Scan {
    pub files: BTreeMap<String, FileInfo>,
    pub dirs: BTreeMap<String, DirInfo>,
    /// Elementi saltati: nomi non validi o che differiscono da un altro solo
    /// per maiuscole (possibile su file system diversi da exFAT).
    pub skipped: Vec<String>,
}

/// Scansiona tutto il disco, esclusi i file ignorati e i link simbolici.
pub fn scan(root: &Path) -> io::Result<Scan> {
    let mut scan = Scan::default();
    walk(root, "", &mut scan)?;
    Ok(scan)
}

/// Visita la cartella `rel`; restituisce `true` se contiene qualcosa.
fn walk(root: &Path, rel: &str, scan: &mut Scan) -> io::Result<bool> {
    let mut entries: Vec<_> = fs::read_dir(paths::native(root, rel))?.collect::<io::Result<_>>()?;
    entries.sort_by_key(|e| e.file_name());
    let mut non_empty = false;
    for entry in entries {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            scan.skipped
                .push(format!("{rel}/{}", name.to_string_lossy()));
            continue;
        };
        if paths::is_ignored(name, rel.is_empty()) {
            continue;
        }
        let child = paths::nfc(&if rel.is_empty() {
            name.to_string()
        } else {
            format!("{rel}/{name}")
        });
        let key = paths::key(&child);
        let meta = fs::symlink_metadata(entry.path())?;
        if meta.is_symlink() {
            continue;
        }
        if scan.files.contains_key(&key) || scan.dirs.contains_key(&key) {
            scan.skipped.push(child);
            continue;
        }
        non_empty = true;
        if meta.is_dir() {
            scan.dirs.insert(
                key.clone(),
                DirInfo {
                    path: child.clone(),
                    empty: false,
                },
            );
            let child_non_empty = walk(root, &child, scan)?;
            if let Some(dir) = scan.dirs.get_mut(&key) {
                dir.empty = !child_non_empty;
            }
        } else if meta.is_file() {
            scan.files.insert(
                key,
                FileInfo {
                    path: child,
                    size: meta.len(),
                    mtime: mtime_ns(&meta)?,
                },
            );
        }
    }
    Ok(non_empty)
}

/// Data di modifica in nanosecondi dal 1970.
pub fn mtime_ns(meta: &Metadata) -> io::Result<i64> {
    Ok(system_time_ns(meta.modified()?))
}

pub fn system_time_ns(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_nanos() as i64,
        Err(e) => -(e.duration().as_nanos() as i64),
    }
}

pub fn ns_system_time(ns: i64) -> SystemTime {
    if ns >= 0 {
        UNIX_EPOCH + std::time::Duration::from_nanos(ns as u64)
    } else {
        UNIX_EPOCH - std::time::Duration::from_nanos(ns.unsigned_abs())
    }
}

/// Due date di modifica uguali, a meno della tolleranza.
pub fn same_mtime(a: i64, b: i64) -> bool {
    (a - b).abs() <= MTIME_TOLERANCE_NS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scans_files_dirs_and_ignores_system_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("Foto/2024/Mare")).unwrap();
        fs::create_dir_all(root.join("Foto/2027")).unwrap();
        fs::create_dir_all(root.join(".hddraid")).unwrap();
        fs::create_dir_all(root.join("$RECYCLE.BIN")).unwrap();
        fs::create_dir_all(root.join("Solo sistema")).unwrap();
        fs::write(root.join("Solo sistema/.DS_Store"), b"x").unwrap();
        fs::write(root.join("Foto/2024/Mare/a.jpg"), b"abc").unwrap();
        fs::write(root.join("Foto/2024/._a.jpg"), b"x").unwrap();
        fs::write(root.join(".hddraid/disk.json"), b"{}").unwrap();
        fs::write(root.join("Scarica HddRaid.html"), b"x").unwrap();

        let scan = scan(root).unwrap();
        let files: Vec<_> = scan.files.values().map(|f| f.path.as_str()).collect();
        assert_eq!(files, ["Foto/2024/Mare/a.jpg"]);
        assert_eq!(scan.files["foto/2024/mare/a.jpg"].size, 3);
        let empty: Vec<_> = scan
            .dirs
            .values()
            .filter(|d| d.empty)
            .map(|d| d.path.as_str())
            .collect();
        assert_eq!(empty, ["Foto/2027", "Solo sistema"]);
        assert!(!scan.dirs["foto"].empty);
    }

    #[test]
    fn paths_are_nfc() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Perche\u{301}.txt"), b"x").unwrap();
        let scan = scan(dir.path()).unwrap();
        let file = scan.files.values().next().unwrap();
        assert_eq!(file.path, "Perch\u{e9}.txt");
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"x").unwrap();
        std::os::unix::fs::symlink(dir.path().join("a.txt"), dir.path().join("b.txt")).unwrap();
        assert_eq!(scan(dir.path()).unwrap().files.len(), 1);
    }

    #[test]
    fn mtime_tolerance() {
        assert!(same_mtime(0, MTIME_TOLERANCE_NS));
        assert!(!same_mtime(0, MTIME_TOLERANCE_NS + 1));
        let t = ns_system_time(-5);
        assert_eq!(system_time_ns(t), -5);
    }
}
