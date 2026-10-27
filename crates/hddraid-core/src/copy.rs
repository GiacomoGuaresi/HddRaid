//! Copia sicura: il file nuovo prende il suo nome solo quando è scritto su
//! disco e verificato. Vedi `doc/formato-dati.md`.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::fsutil::sync_parent;
use crate::hash::{BUF_SIZE, Hasher, hash_file};
use crate::paths::PARTIAL_SUFFIX;
use crate::scan::{mtime_ns, ns_system_time};

/// Risultato di una copia riuscita.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    pub hash: String,
    pub size: u64,
    /// Data di modifica, uguale a quella della sorgente.
    pub mtime: i64,
}

/// Copia `src` in `dst`:
///
/// 1. scrive `.<nome>.hddraid-partial` nella cartella di destinazione,
///    calcolando l'hash della sorgente mentre la legge (una lettura sola);
/// 2. `fsync`;
/// 3. rilegge la copia e ne confronta l'hash;
/// 4. imposta la data di modifica della sorgente;
/// 5. rinomina al nome definitivo, sostituendo in un passo l'eventuale file
///    esistente, che fino a quel momento resta intatto.
///
/// Se `expected_hash` è indicato e la sorgente non corrisponde, oppure se la
/// sorgente cambia durante la copia, restituisce [`Error::SourceChanged`] e
/// non tocca la destinazione. `progress` riceve i byte copiati.
pub fn safe_copy(
    src: &Path,
    dst: &Path,
    expected_hash: Option<&str>,
    progress: &mut dyn FnMut(u64),
) -> Result<Copied> {
    let before = fs::metadata(src)?;
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let partial = partial_path(dst);
    let _ = fs::remove_file(&partial);
    let result = write_and_verify(src, &partial, &before, expected_hash, progress);
    match result {
        Ok(copied) => {
            fs::rename(&partial, dst)?;
            sync_parent(dst);
            Ok(copied)
        }
        Err(e) => {
            let _ = fs::remove_file(&partial);
            Err(e)
        }
    }
}

fn write_and_verify(
    src: &Path,
    partial: &Path,
    before: &fs::Metadata,
    expected_hash: Option<&str>,
    progress: &mut dyn FnMut(u64),
) -> Result<Copied> {
    let mut reader = File::open(src)?;
    let mut writer = File::create(partial)?;
    let mut hasher = Hasher::new();
    let mut buf = vec![0u8; BUF_SIZE];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        writer.write_all(&buf[..n])?;
        progress(n as u64);
    }
    writer.sync_all()?;
    drop(writer);
    let hash = hasher.finish();

    let after = fs::metadata(src)?;
    let mtime = mtime_ns(before)?;
    if after.len() != before.len() || mtime_ns(&after)? != mtime {
        return Err(Error::SourceChanged(src.to_path_buf()));
    }
    if expected_hash.is_some_and(|h| h != hash) {
        return Err(Error::SourceChanged(src.to_path_buf()));
    }
    // Rilettura. Su Linux potrebbe arrivare dalla cache del sistema invece che
    // dal disco: resta comunque un controllo di quanto è stato scritto.
    if hash_file(partial)? != hash {
        return Err(Error::VerifyFailed(partial.to_path_buf()));
    }
    OpenOptions::new()
        .write(true)
        .open(partial)?
        .set_modified(ns_system_time(mtime))?;
    Ok(Copied {
        hash,
        size: before.len(),
        mtime,
    })
}

/// `.<nome>.hddraid-partial` accanto a `dst`.
pub fn partial_path(dst: &Path) -> PathBuf {
    let name = dst
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    dst.with_file_name(format!(".{name}{PARTIAL_SUFFIX}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash_reader;

    fn no_progress() -> impl FnMut(u64) {
        |_| {}
    }

    #[test]
    fn copies_verifies_and_keeps_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.jpg");
        let dst = dir.path().join("b/c/a.jpg");
        fs::write(&src, b"contenuto").unwrap();
        let old = ns_system_time(1_600_000_000_000_000_000);
        OpenOptions::new()
            .write(true)
            .open(&src)
            .unwrap()
            .set_modified(old)
            .unwrap();

        let mut total = 0;
        let copied = safe_copy(&src, &dst, None, &mut |n| total += n).unwrap();
        assert_eq!(fs::read(&dst).unwrap(), b"contenuto");
        assert_eq!(copied.hash, hash_reader(&b"contenuto"[..]).unwrap());
        assert_eq!(copied.size, 9);
        assert_eq!(total, 9);
        assert_eq!(fs::metadata(&dst).unwrap().modified().unwrap(), old);
        assert!(!partial_path(&dst).exists());
    }

    #[test]
    fn replaces_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("b.txt");
        fs::write(&src, b"nuovo").unwrap();
        fs::write(&dst, b"vecchio").unwrap();
        safe_copy(&src, &dst, None, &mut no_progress()).unwrap();
        assert_eq!(fs::read(&dst).unwrap(), b"nuovo");
    }

    #[test]
    fn wrong_expected_hash_leaves_destination_alone() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("b.txt");
        fs::write(&src, b"nuovo").unwrap();
        fs::write(&dst, b"vecchio").unwrap();
        let err = safe_copy(&src, &dst, Some("00"), &mut no_progress()).unwrap_err();
        assert!(matches!(err, Error::SourceChanged(_)));
        assert_eq!(fs::read(&dst).unwrap(), b"vecchio");
        assert!(!partial_path(&dst).exists());
    }

    #[test]
    fn stale_partial_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("b.txt");
        fs::write(&src, b"nuovo").unwrap();
        fs::write(partial_path(&dst), b"copia interrotta").unwrap();
        safe_copy(&src, &dst, None, &mut no_progress()).unwrap();
        assert_eq!(fs::read(&dst).unwrap(), b"nuovo");
        assert!(!partial_path(&dst).exists());
    }

    #[test]
    fn missing_source_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = safe_copy(
            &dir.path().join("manca"),
            &dir.path().join("b"),
            None,
            &mut no_progress(),
        )
        .unwrap_err();
        assert!(matches!(err, Error::Io(_)));
    }
}
