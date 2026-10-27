//! Indice: `.hddraid/index.sqlite`, uguale su entrambi i dischi.
//!
//! Contiene la *base* (i file com'erano all'ultimo allineamento), le verifiche
//! del controllo completo, lo storico e il journal delle operazioni in corso.
//! Vedi `doc/formato-dati.md`.

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, OpenOptions};
use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};

use crate::error::{Error, Result};
use crate::fsutil::{sibling_tmp, sync_parent};

/// Versione dello schema, in `PRAGMA user_version`.
pub const SCHEMA_VERSION: i32 = 1;

/// Algoritmo di hash usato in tutto l'indice.
pub const HASH_ALGORITHM: &str = "sha256";

const SCHEMA: &str = "
CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE disks (
    disk_id TEXT PRIMARY KEY,
    label   TEXT NOT NULL,
    added   TEXT NOT NULL,
    removed TEXT
);
CREATE TABLE files (
    key   TEXT PRIMARY KEY,
    path  TEXT NOT NULL,
    kind  TEXT NOT NULL CHECK (kind IN ('file', 'dir')),
    size  INTEGER NOT NULL,
    mtime INTEGER NOT NULL,
    hash  TEXT,
    added TEXT NOT NULL
);
CREATE TABLE verified (
    key     TEXT NOT NULL,
    disk_id TEXT NOT NULL,
    at      TEXT NOT NULL,
    PRIMARY KEY (key, disk_id)
);
CREATE TABLE events (
    id      INTEGER PRIMARY KEY,
    at      TEXT NOT NULL,
    kind    TEXT NOT NULL,
    path    TEXT,
    disk_id TEXT,
    details TEXT
);
CREATE TABLE journal (
    id     INTEGER PRIMARY KEY,
    at     TEXT NOT NULL,
    op     TEXT NOT NULL,
    params TEXT NOT NULL
);
";

/// Tipo di una voce della base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    /// Cartella vuota: le cartelle con dei file non si tracciano.
    Dir,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::File => "file",
            Kind::Dir => "dir",
        }
    }

    fn parse(s: &str) -> Kind {
        if s == "dir" { Kind::Dir } else { Kind::File }
    }
}

/// Un file (o una cartella vuota) com'era all'ultimo allineamento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseEntry {
    /// Percorso relativo alla radice, con `/`, in NFC.
    pub path: String,
    pub kind: Kind,
    pub size: u64,
    /// Data di modifica, in nanosecondi dal 1970 (UTC).
    pub mtime: i64,
    /// SHA-256 in esadecimale; assente per le cartelle.
    pub hash: Option<String>,
}

/// La base: per ogni chiave di percorso (vedi `paths::key`), la sua voce.
pub type Base = BTreeMap<String, BaseEntry>;

/// Un evento dello storico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub at: String,
    pub kind: String,
    pub path: Option<String>,
    pub disk_id: Option<String>,
    pub details: Option<String>,
}

/// Un'operazione rimasta nel journal: è stata interrotta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub id: i64,
    pub at: String,
    pub op: String,
    pub params: String,
}

/// Indice aperto.
pub struct Index {
    conn: Connection,
}

impl Index {
    /// Crea un indice nuovo e vuoto. Rifiuta un file già esistente.
    pub fn create(path: &Path, archive_id: &str) -> Result<Index> {
        if path.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("{} esiste già", path.display()),
            )));
        }
        let mut index = Index {
            conn: open_conn(path)?,
        };
        let now = crate::now_rfc3339();
        let tx = index.conn.transaction()?;
        tx.execute_batch(SCHEMA)?;
        for (key, value) in [
            ("archive_id", archive_id),
            ("hash_algorithm", HASH_ALGORITHM),
            ("revision", "0"),
            ("created", &now),
        ] {
            tx.execute(
                "INSERT INTO meta (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
        }
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        tx.commit()?;
        Ok(index)
    }

    /// Apre un indice esistente, applicando le migrazioni mancanti.
    pub fn open(path: &Path) -> Result<Index> {
        if !path.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} non esiste", path.display()),
            )));
        }
        let index = Index {
            conn: open_conn(path)?,
        };
        let version: i32 = index
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(Error::IndexTooNew {
                found: version,
                supported: SCHEMA_VERSION,
            });
        }
        if version < 1 {
            return Err(Error::IndexInvalid(path.to_path_buf()));
        }
        // Qui andranno le migrazioni da `version` a `SCHEMA_VERSION`.
        Ok(index)
    }

    /// Copia l'indice `src` (aperto in sola lettura) in `dest` e apre la copia.
    pub fn copy_from(src: &Path, dest: &Path) -> Result<Index> {
        Index::open(src)?;
        let conn = Connection::open_with_flags(src, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Index { conn }.save_to(dest)?;
        Index::open(dest)
    }

    /// Legge `archive_id` e `revision` di un indice senza modificarlo, per
    /// scegliere il più recente tra quelli dei due dischi.
    pub fn peek(path: &Path) -> Result<(String, u64)> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let get = |key: &str| -> Result<String> {
            Ok(conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))?)
        };
        let revision = get("revision")?
            .parse()
            .map_err(|_| Error::IndexInvalid(path.to_path_buf()))?;
        Ok((get("archive_id")?, revision))
    }

    /// Copia l'indice in `dest`: backup SQLite su un file temporaneo, `fsync`,
    /// rinomina atomica. Un'interruzione lascia l'indice vecchio intatto.
    pub fn save_to(&self, dest: &Path) -> Result<()> {
        let tmp = sibling_tmp(dest);
        let _ = fs::remove_file(&tmp);
        self.conn.backup(
            rusqlite::MAIN_DB,
            &tmp,
            None::<fn(rusqlite::backup::Progress)>,
        )?;
        // Su Windows `sync_all` richiede un file aperto in scrittura.
        OpenOptions::new().write(true).open(&tmp)?.sync_all()?;
        fs::rename(&tmp, dest)?;
        sync_parent(dest);
        Ok(())
    }

    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn archive_id(&self) -> Result<String> {
        self.meta("archive_id")?
            .ok_or_else(|| Error::IndexInvalid(self.conn.path().unwrap_or_default().into()))
    }

    /// Numero di scritture fatte sull'indice: cresce a ogni [`Index::update`].
    pub fn revision(&self) -> Result<u64> {
        Ok(self
            .meta("revision")?
            .and_then(|r| r.parse().ok())
            .unwrap_or(0))
    }

    /// La base completa.
    pub fn base(&self) -> Result<Base> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, path, kind, size, mtime, hash FROM files")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                BaseEntry {
                    path: r.get(1)?,
                    kind: Kind::parse(&r.get::<_, String>(2)?),
                    size: r.get::<_, i64>(3)? as u64,
                    mtime: r.get(4)?,
                    hash: r.get(5)?,
                },
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Chiavi verificate su un disco a partire da `since` (RFC 3339).
    pub fn verified_since(&self, disk_id: &str, since: &str) -> Result<HashSet<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key FROM verified WHERE disk_id = ?1 AND at >= ?2")?;
        let rows = stmt.query_map([disk_id, since], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Ultimi eventi dello storico, dal più recente.
    pub fn events(&self, limit: usize) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT at, kind, path, disk_id, details FROM events ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |r| {
            Ok(Event {
                at: r.get(0)?,
                kind: r.get(1)?,
                path: r.get(2)?,
                disk_id: r.get(3)?,
                details: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Operazioni rimaste nel journal.
    pub fn journal(&self) -> Result<Vec<JournalEntry>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, at, op, params FROM journal ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok(JournalEntry {
                id: r.get(0)?,
                at: r.get(1)?,
                op: r.get(2)?,
                params: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Scrive sull'indice in una transazione, che incrementa `revision`.
    pub fn update<T>(&mut self, f: impl FnOnce(&Tx) -> Result<T>) -> Result<T> {
        let tx = Tx(self.conn.transaction()?);
        let out = f(&tx)?;
        tx.0.execute(
            "UPDATE meta SET value = CAST(value AS INTEGER) + 1 WHERE key = 'revision'",
            [],
        )?;
        tx.0.commit()?;
        Ok(out)
    }
}

/// Transazione di scrittura sull'indice, vedi [`Index::update`].
pub struct Tx<'a>(Transaction<'a>);

impl Tx<'_> {
    pub fn set_meta(&self, key: &str, value: Option<&str>) -> Result<()> {
        match value {
            Some(v) => self.0.execute(
                "INSERT INTO meta (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                params![key, v],
            )?,
            None => self.0.execute("DELETE FROM meta WHERE key = ?1", [key])?,
        };
        Ok(())
    }

    pub fn add_disk(&self, disk_id: &str, label: &str) -> Result<()> {
        self.0.execute(
            "INSERT INTO disks (disk_id, label, added) VALUES (?1, ?2, ?3)
             ON CONFLICT (disk_id) DO UPDATE SET label = excluded.label",
            params![disk_id, label, crate::now_rfc3339()],
        )?;
        Ok(())
    }

    /// Inserisce o sostituisce una voce della base.
    pub fn put(&self, key: &str, entry: &BaseEntry) -> Result<()> {
        self.0.execute(
            "INSERT INTO files (key, path, kind, size, mtime, hash, added)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (key) DO UPDATE SET path = excluded.path, kind = excluded.kind,
                 size = excluded.size, mtime = excluded.mtime, hash = excluded.hash",
            params![
                key,
                entry.path,
                entry.kind.as_str(),
                entry.size as i64,
                entry.mtime,
                entry.hash,
                crate::now_rfc3339()
            ],
        )?;
        Ok(())
    }

    /// Toglie una voce dalla base (e le sue verifiche).
    pub fn forget(&self, key: &str) -> Result<()> {
        self.0.execute("DELETE FROM files WHERE key = ?1", [key])?;
        self.0
            .execute("DELETE FROM verified WHERE key = ?1", [key])?;
        Ok(())
    }

    pub fn set_verified(&self, key: &str, disk_id: &str, at: &str) -> Result<()> {
        self.0.execute(
            "INSERT INTO verified (key, disk_id, at) VALUES (?1, ?2, ?3)
             ON CONFLICT (key, disk_id) DO UPDATE SET at = excluded.at",
            params![key, disk_id, at],
        )?;
        Ok(())
    }

    pub fn log(
        &self,
        kind: &str,
        path: Option<&str>,
        disk_id: Option<&str>,
        details: Option<&str>,
    ) -> Result<()> {
        self.0.execute(
            "INSERT INTO events (at, kind, path, disk_id, details) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![crate::now_rfc3339(), kind, path, disk_id, details],
        )?;
        Ok(())
    }

    /// Registra un'operazione prima di toccare i file. Restituisce l'id da
    /// passare a [`Tx::journal_end`] quando l'operazione è finita.
    pub fn journal_begin(&self, op: &str, params: &str) -> Result<i64> {
        self.0.execute(
            "INSERT INTO journal (at, op, params) VALUES (?1, ?2, ?3)",
            params![crate::now_rfc3339(), op, params],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn journal_end(&self, id: i64) -> Result<()> {
        self.0.execute("DELETE FROM journal WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn journal_clear(&self) -> Result<()> {
        self.0.execute("DELETE FROM journal", [])?;
        Ok(())
    }
}

fn open_conn(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    // Niente WAL: usa memoria condivisa, poco affidabile su dischi rimovibili.
    conn.pragma_update(None, "journal_mode", "DELETE")?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> BaseEntry {
        BaseEntry {
            path: path.into(),
            kind: Kind::File,
            size: 3,
            mtime: 1_700_000_000_000_000_000,
            hash: Some("abc".into()),
        }
    }

    #[test]
    fn create_open_and_base() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.sqlite");
        let mut index = Index::create(&path, "arch-1").unwrap();
        assert_eq!(index.revision().unwrap(), 0);
        index
            .update(|tx| {
                tx.put("foto/a.jpg", &entry("Foto/a.jpg"))?;
                tx.put("foto/b.jpg", &entry("Foto/b.jpg"))
            })
            .unwrap();
        index.update(|tx| tx.forget("foto/b.jpg")).unwrap();
        drop(index);

        let index = Index::open(&path).unwrap();
        assert_eq!(index.archive_id().unwrap(), "arch-1");
        assert_eq!(index.revision().unwrap(), 2);
        let base = index.base().unwrap();
        assert_eq!(base.len(), 1);
        assert_eq!(base["foto/a.jpg"], entry("Foto/a.jpg"));
    }

    #[test]
    fn create_refuses_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.sqlite");
        Index::create(&path, "arch-1").unwrap();
        assert!(Index::create(&path, "arch-1").is_err());
    }

    #[test]
    fn newer_schema_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.sqlite");
        let index = Index::create(&path, "arch-1").unwrap();
        index
            .conn
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        drop(index);
        assert!(matches!(Index::open(&path), Err(Error::IndexTooNew { .. })));
    }

    #[test]
    fn garbage_file_is_not_an_index() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.sqlite");
        fs::write(&path, b"non sono un database, solo testo qualsiasi").unwrap();
        assert!(Index::open(&path).is_err());
        assert!(Index::peek(&path).is_err());
    }

    #[test]
    fn save_to_and_peek() {
        let dir = tempfile::tempdir().unwrap();
        let mut index = Index::create(&dir.path().join("work.sqlite"), "arch-1").unwrap();
        index.update(|tx| tx.put("a", &entry("a"))).unwrap();
        let dest = dir.path().join("disk.sqlite");
        index.save_to(&dest).unwrap();
        assert_eq!(Index::peek(&dest).unwrap(), ("arch-1".into(), 1));
        assert_eq!(Index::open(&dest).unwrap().base().unwrap().len(), 1);
        assert!(!sibling_tmp(&dest).exists());
    }

    #[test]
    fn journal_and_events() {
        let dir = tempfile::tempdir().unwrap();
        let mut index = Index::create(&dir.path().join("i.sqlite"), "arch-1").unwrap();
        let id = index
            .update(|tx| tx.journal_begin("copy", r#"{"path":"a"}"#))
            .unwrap();
        assert_eq!(index.journal().unwrap().len(), 1);
        index
            .update(|tx| {
                tx.journal_end(id)?;
                tx.log("copied", Some("a"), Some("disk-b"), None)
            })
            .unwrap();
        assert!(index.journal().unwrap().is_empty());
        assert_eq!(index.events(10).unwrap()[0].kind, "copied");
    }

    #[test]
    fn verified_since_filters_by_date() {
        let dir = tempfile::tempdir().unwrap();
        let mut index = Index::create(&dir.path().join("i.sqlite"), "arch-1").unwrap();
        index
            .update(|tx| {
                tx.set_verified("a", "d1", "2026-01-01T00:00:00Z")?;
                tx.set_verified("b", "d1", "2026-06-01T00:00:00Z")?;
                tx.set_verified("b", "d2", "2026-06-01T00:00:00Z")
            })
            .unwrap();
        let v = index.verified_since("d1", "2026-03-01T00:00:00Z").unwrap();
        assert_eq!(v, HashSet::from(["b".to_string()]));
    }
}
