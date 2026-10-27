//! Motore di HddRaid: dischi, indice, sincronizzazione e controllo completo.
//!
//! Non dipende dall'interfaccia: l'app Tauri si limita a chiamarlo.

/// Versione del motore.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
