//! Motore di HddRaid: dischi, indice, sincronizzazione e controllo completo.
//!
//! Non dipende dall'interfaccia: l'app Tauri si limita a chiamarlo.

pub mod archive;
pub mod check;
pub mod copy;
pub mod disk;
pub mod error;
pub mod fsutil;
pub mod hash;
pub mod index;
pub mod paths;
pub mod plan;
pub mod scan;
pub mod sync;

pub use archive::{Archive, Side};
pub use disk::Disk;
pub use error::{Error, Result};

/// Versione del motore.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Data e ora attuali in UTC, in formato RFC 3339 con i millisecondi. Il
/// formato è fisso, quindi le date si possono confrontare come stringhe.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
