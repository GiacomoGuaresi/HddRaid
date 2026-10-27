use std::io;
use std::path::PathBuf;

/// Errori del motore.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("errore di lettura o scrittura: {0}")]
    Io(#[from] io::Error),

    #[error("file dell'app non valido: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{} è già un disco HddRaid", .0.display())]
    AlreadyInitialized(PathBuf),

    #[error("{} non è un disco HddRaid", .0.display())]
    NotADisk(PathBuf),

    #[error("formato {found} scritto da un'app più nuova (supportato fino a {supported})")]
    FormatTooNew { found: u32, supported: u32 },

    #[error("errore dell'indice: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("indice scritto da un'app più nuova (schema {found}, supportato fino a {supported})")]
    IndexTooNew { found: i32, supported: i32 },

    #[error("{} non è un indice HddRaid valido", .0.display())]
    IndexInvalid(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;
