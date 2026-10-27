//! Utilità per scritture sicure sui dischi.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Scrive un file in modo atomico: file temporaneo → `fsync` → rinomina.
/// Un'interruzione lascia il file vecchio o quello nuovo, mai uno a metà.
pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = sibling_tmp(path);
    {
        let mut file = File::create(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    sync_parent(path);
    Ok(())
}

/// Percorso temporaneo nella stessa cartella: `.nome.tmp`.
pub fn sibling_tmp(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.tmp"))
}

/// Forza su disco la cartella che contiene `path`, così una rinomina appena
/// fatta sopravvive a uno spegnimento. Non tutti i file system lo permettono
/// (e su Windows non serve): gli errori si ignorano.
pub fn sync_parent(path: &Path) {
    #[cfg(unix)]
    if let Some(Ok(dir)) = path.parent().map(File::open) {
        let _ = dir.sync_all();
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_and_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.json");
        atomic_write(&path, b"uno").unwrap();
        atomic_write(&path, b"due").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"due");
        assert!(!sibling_tmp(&path).exists());
    }
}
