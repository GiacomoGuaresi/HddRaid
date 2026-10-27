//! Motore di HddRaid: dischi, indice, sincronizzazione e controllo completo.
//!
//! Non dipende dall'interfaccia: l'app Tauri si limita a chiamarlo.

pub mod disk;
pub mod error;
pub mod fsutil;
pub mod hash;
pub mod index;
pub mod paths;
pub mod scan;

pub use disk::Disk;
pub use error::{Error, Result};

/// Versione del motore.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Data e ora attuali in UTC, in formato RFC 3339.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
