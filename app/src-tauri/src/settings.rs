//! Impostazioni dell'app sul computer: `settings.json` nella cartella dati.

use std::fs;
use std::path::{Path, PathBuf};

use hddraid_core::Side;
use serde::{Deserialize, Serialize};

/// Le impostazioni che l'utente vede.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub autostart: bool,
    /// Mesi tra un controllo completo e l'altro.
    pub check_months: u32,
    /// `None`: lingua del sistema.
    pub language: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            autostart: true,
            check_months: 3,
            language: None,
        }
    }
}

/// Un disco della coppia, ricordato anche quando non è collegato.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownDisk {
    pub id: String,
    pub label: String,
}

/// La coppia di dischi usata su questo computer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pair {
    pub archive_id: String,
    pub disks: [KnownDisk; 2],
}

impl Pair {
    pub fn label(&self, side: Side) -> &str {
        &self.disks[side as usize].label
    }

    /// Nome del disco con questo identificativo.
    pub fn label_of(&self, disk_id: &str) -> Option<&str> {
        self.disks
            .iter()
            .find(|d| d.id == disk_id)
            .map(|d| d.label.as_str())
    }
}

/// Tutto ciò che si salva in `settings.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub settings: Settings,
    #[serde(default)]
    pub pair: Option<Pair>,
    /// "Più tardi" sul controllo completo: non riproporlo prima di questa data.
    #[serde(default)]
    pub check_postponed_until: Option<String>,
    /// L'avvio automatico è già stato impostato la prima volta.
    #[serde(default)]
    pub autostart_configured: bool,
}

impl Config {
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("settings.json")
    }

    /// Legge le impostazioni; se mancano o sono illeggibili, usa quelle di
    /// default.
    pub fn load(dir: &Path) -> Config {
        match fs::read(Self::path(dir)) {
            Ok(data) => serde_json::from_slice(&data).unwrap_or_else(|e| {
                log::warn!("settings.json illeggibile, uso i default: {e}");
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self, dir: &Path) -> hddraid_core::Result<()> {
        fs::create_dir_all(dir)?;
        let json = serde_json::to_vec_pretty(self)?;
        hddraid_core::fsutil::atomic_write(&Self::path(dir), &json)?;
        Ok(())
    }
}
