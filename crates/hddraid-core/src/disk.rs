//! Identità di un disco: `.hddraid/disk.json`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::archive::Side;
use crate::error::{Error, Result};
use crate::fsutil::atomic_write;

/// Cartella dell'app nella radice di ogni disco.
pub const DATA_DIR: &str = ".hddraid";
/// Versione del formato di `disk.json`.
pub const FORMAT: u32 = 1;

const IDENTITY_FILE: &str = "disk.json";
const INDEX_FILE: &str = "index.sqlite";

/// Contenuto di `Scarica HddRaid.html`.
const DOWNLOAD_HTML: &str = include_str!("../assets/download.html");

/// Contenuto di `disk.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskIdentity {
    /// Identificativo unico del disco.
    pub disk_id: String,
    /// Identificativo della coppia, uguale su entrambi i dischi.
    pub archive_id: String,
    /// Il lato nella coppia: resta lo stesso su qualsiasi computer.
    pub side: Side,
    /// Nome scelto dall'utente.
    pub label: String,
    /// Data di preparazione (UTC, RFC 3339).
    pub created: String,
    /// Versione del formato.
    pub format: u32,
}

/// Un disco HddRaid collegato.
#[derive(Debug, Clone)]
pub struct Disk {
    root: PathBuf,
    identity: DiskIdentity,
}

impl Disk {
    /// Prepara un disco: scrive `.hddraid/disk.json`. I file già presenti
    /// restano dove sono. Rifiuta un disco già preparato.
    pub fn init(root: &Path, archive_id: &str, side: Side, label: &str) -> Result<Disk> {
        let path = identity_path(root);
        if path.exists() {
            return Err(Error::AlreadyInitialized(root.to_path_buf()));
        }
        fs::create_dir_all(root.join(DATA_DIR))?;
        let identity = DiskIdentity {
            disk_id: uuid::Uuid::new_v4().to_string(),
            archive_id: archive_id.to_string(),
            side,
            label: label.to_string(),
            created: crate::now_rfc3339(),
            format: FORMAT,
        };
        let disk = Disk {
            root: root.to_path_buf(),
            identity,
        };
        disk.write_identity()?;
        hide_data_dir(root);
        disk.write_download_page()?;
        Ok(disk)
    }

    /// Apre un disco già preparato.
    pub fn open(root: &Path) -> Result<Disk> {
        Self::detect(root)?.ok_or_else(|| Error::NotADisk(root.to_path_buf()))
    }

    /// Come [`Disk::open`], ma un disco senza `disk.json` non è un errore.
    pub fn detect(root: &Path) -> Result<Option<Disk>> {
        let data = match fs::read(identity_path(root)) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let identity: DiskIdentity = serde_json::from_slice(&data)?;
        if identity.format > FORMAT {
            return Err(Error::FormatTooNew {
                found: identity.format,
                supported: FORMAT,
            });
        }
        Ok(Some(Disk {
            root: root.to_path_buf(),
            identity,
        }))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn identity(&self) -> &DiskIdentity {
        &self.identity
    }

    pub fn id(&self) -> &str {
        &self.identity.disk_id
    }

    pub fn side(&self) -> Side {
        self.identity.side
    }

    pub fn label(&self) -> &str {
        &self.identity.label
    }

    /// Cambia il nome del disco.
    pub fn set_label(&mut self, label: &str) -> Result<()> {
        self.identity.label = label.to_string();
        self.write_identity()
    }

    pub fn data_dir(&self) -> PathBuf {
        self.root.join(DATA_DIR)
    }

    /// Percorso dell'indice su questo disco.
    pub fn index_path(&self) -> PathBuf {
        self.data_dir().join(INDEX_FILE)
    }

    /// Scrive `Scarica HddRaid.html` nella radice, se manca o è vecchia.
    pub fn write_download_page(&self) -> Result<()> {
        let path = self.root.join(crate::paths::DOWNLOAD_PAGE);
        if fs::read(&path).is_ok_and(|current| current == DOWNLOAD_HTML.as_bytes()) {
            return Ok(());
        }
        atomic_write(&path, DOWNLOAD_HTML.as_bytes())?;
        Ok(())
    }

    fn write_identity(&self) -> Result<()> {
        let json = serde_json::to_vec_pretty(&self.identity)?;
        atomic_write(&identity_path(&self.root), &json)?;
        Ok(())
    }
}

fn identity_path(root: &Path) -> PathBuf {
    root.join(DATA_DIR).join(IDENTITY_FILE)
}

/// Su Windows i file che iniziano con il punto non sono nascosti: si marca la
/// cartella dell'app come nascosta. Altrove non serve.
fn hide_data_dir(root: &Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("attrib")
            .arg("+h")
            .arg(root.join(DATA_DIR))
            .status();
    }
    #[cfg(not(windows))]
    let _ = root;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_page_is_written_once() {
        let dir = tempfile::tempdir().unwrap();
        let disk = Disk::init(dir.path(), "arch-1", Side::A, "Disco A").unwrap();
        let page = dir.path().join(crate::paths::DOWNLOAD_PAGE);
        assert!(fs::read_to_string(&page).unwrap().contains("HddRaid"));
        fs::remove_file(&page).unwrap();
        disk.write_download_page().unwrap();
        assert!(page.exists());
    }

    #[test]
    fn init_then_open() {
        let dir = tempfile::tempdir().unwrap();
        let disk = Disk::init(dir.path(), "arch-1", Side::A, "Disco A").unwrap();
        let opened = Disk::open(dir.path()).unwrap();
        assert_eq!(disk.identity(), opened.identity());
        assert_eq!(opened.label(), "Disco A");
        assert_eq!(opened.identity().archive_id, "arch-1");
    }

    #[test]
    fn init_twice_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        Disk::init(dir.path(), "arch-1", Side::A, "Disco A").unwrap();
        let err = Disk::init(dir.path(), "arch-2", Side::B, "Disco B").unwrap_err();
        assert!(matches!(err, Error::AlreadyInitialized(_)));
    }

    #[test]
    fn init_keeps_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("foto.jpg"), b"x").unwrap();
        Disk::init(dir.path(), "arch-1", Side::A, "Disco A").unwrap();
        assert_eq!(fs::read(dir.path().join("foto.jpg")).unwrap(), b"x");
    }

    #[test]
    fn detect_plain_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Disk::detect(dir.path()).unwrap().is_none());
        assert!(matches!(Disk::open(dir.path()), Err(Error::NotADisk(_))));
    }

    #[test]
    fn newer_format_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut disk = Disk::init(dir.path(), "arch-1", Side::A, "Disco A").unwrap();
        disk.identity.format = FORMAT + 1;
        disk.write_identity().unwrap();
        assert!(matches!(
            Disk::open(dir.path()),
            Err(Error::FormatTooNew { .. })
        ));
    }

    #[test]
    fn set_label_persists() {
        let dir = tempfile::tempdir().unwrap();
        let mut disk = Disk::init(dir.path(), "arch-1", Side::A, "Disco A").unwrap();
        disk.set_label("Disco rosso").unwrap();
        assert_eq!(Disk::open(dir.path()).unwrap().label(), "Disco rosso");
    }
}
