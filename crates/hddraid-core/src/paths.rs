//! Percorsi: forma canonica, chiavi di confronto, file da ignorare.
//!
//! Nell'indice i percorsi sono relativi alla radice del disco, separati da
//! `/` e in Unicode NFC (macOS scrive gli accenti in NFD, Windows in NFC).
//! Si confrontano tramite una *chiave* che ignora anche maiuscole e minuscole,
//! come fa exFAT.

use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

use crate::disk::DATA_DIR;

/// Pagina che rimanda al download dell'app, nella radice di ogni disco.
pub const DOWNLOAD_PAGE: &str = "Scarica HddRaid.html";

/// Suffisso delle copie in corso (vedi `copy`).
pub const PARTIAL_SUFFIX: &str = ".hddraid-partial";

/// Nomi di sistema ignorati a qualsiasi livello (confronto senza maiuscole).
const IGNORED_NAMES: &[&str] = &[
    // Windows
    "$RECYCLE.BIN",
    "System Volume Information",
    "Thumbs.db",
    "desktop.ini",
    // macOS
    ".Trashes",
    ".Spotlight-V100",
    ".fseventsd",
    ".TemporaryItems",
    ".DocumentRevisions-V100",
    ".DS_Store",
];

/// Prefissi ignorati: file AppleDouble di macOS e cestini di Linux.
const IGNORED_PREFIXES: &[&str] = &["._", ".Trash-"];

/// Forma canonica di un percorso: NFC.
pub fn nfc(path: &str) -> String {
    path.nfc().collect()
}

/// Chiave di confronto: NFC e minuscole.
pub fn key(path: &str) -> String {
    nfc(path).to_lowercase()
}

/// `true` se il nome va ignorato. `at_root` indica che è nella radice del
/// disco, dove stanno anche i file dell'app.
pub fn is_ignored(name: &str, at_root: bool) -> bool {
    if at_root && (name == DATA_DIR || name == DOWNLOAD_PAGE) {
        return true;
    }
    IGNORED_NAMES.iter().any(|n| n.eq_ignore_ascii_case(name))
        || IGNORED_PREFIXES.iter().any(|p| name.starts_with(p))
        || name.ends_with(PARTIAL_SUFFIX)
}

/// Percorso sul disco a partire da quello relativo.
pub fn native(root: &Path, rel: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    path.extend(rel.split('/'));
    path
}

/// Cartella che contiene `rel`, se non è nella radice.
pub fn parent(rel: &str) -> Option<&str> {
    rel.rsplit_once('/').map(|(p, _)| p)
}

/// Ultimo componente di `rel`.
pub fn file_name(rel: &str) -> &str {
    rel.rsplit_once('/').map_or(rel, |(_, n)| n)
}

/// `rel` con un altro nome nella stessa cartella.
pub fn with_file_name(rel: &str, name: &str) -> String {
    match parent(rel) {
        Some(p) => format!("{p}/{name}"),
        None => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_ignores_case_and_normalization() {
        let nfd = "Perche\u{301}.jpg";
        let nfc = "Perch\u{e9}.JPG";
        assert_ne!(nfd, nfc);
        assert_eq!(key(nfd), key(nfc));
        assert_eq!(super::nfc(nfd), "Perch\u{e9}.jpg");
    }

    #[test]
    fn ignored_names() {
        for name in [
            ".DS_Store",
            "thumbs.db",
            "$RECYCLE.BIN",
            "._IMG_0001.jpg",
            ".Trash-1000",
            ".a.jpg.hddraid-partial",
        ] {
            assert!(is_ignored(name, false), "{name}");
        }
        assert!(is_ignored(".hddraid", true));
        assert!(is_ignored("Scarica HddRaid.html", true));
        assert!(!is_ignored(".hddraid", false));
        assert!(!is_ignored("IMG_0001.jpg", true));
        assert!(!is_ignored(".config", false));
    }

    #[test]
    fn path_helpers() {
        assert_eq!(native(Path::new("/d"), "a/b.jpg"), Path::new("/d/a/b.jpg"));
        assert_eq!(parent("a/b/c.jpg"), Some("a/b"));
        assert_eq!(parent("c.jpg"), None);
        assert_eq!(file_name("a/b/c.jpg"), "c.jpg");
        assert_eq!(with_file_name("a/c.jpg", "d.jpg"), "a/d.jpg");
        assert_eq!(with_file_name("c.jpg", "d.jpg"), "d.jpg");
    }
}
