//! La coppia di dischi e il suo indice.
//!
//! L'app lavora su una copia locale dell'indice (in `work_dir`) e la riscrive
//! su entrambi i dischi con [`Archive::save`]: così un disco scollegato non
//! lascia un database aperto a metà.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::disk::Disk;
use crate::error::{Error, Result};
use crate::index::{Index, JournalEntry};

/// Uno dei due dischi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Side {
    A,
    B,
}

impl Side {
    pub const BOTH: [Side; 2] = [Side::A, Side::B];

    pub fn other(self) -> Side {
        match self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }

    fn idx(self) -> usize {
        match self {
            Side::A => 0,
            Side::B => 1,
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Side::A => "A",
            Side::B => "B",
        })
    }
}

/// Cosa è successo all'apertura.
#[derive(Debug, Default)]
pub struct OpenInfo {
    /// L'indice mancava o era illeggibile su entrambi i dischi ed è stato
    /// ricreato vuoto: il prossimo allineamento sarà un'adozione.
    pub index_rebuilt: bool,
    /// Revisioni dell'indice trovate sui dischi A e B (`None` se illeggibile).
    pub revisions: [Option<u64>; 2],
    /// Operazioni interrotte rimaste nel journal.
    pub interrupted: Vec<JournalEntry>,
}

/// Coppia di dischi aperta.
pub struct Archive {
    disks: [Disk; 2],
    index: Index,
    info: OpenInfo,
}

impl Archive {
    /// Prepara due dischi come una nuova coppia. I file già presenti restano
    /// dove sono: il primo allineamento li unirà.
    pub fn create(
        root_a: &Path,
        root_b: &Path,
        labels: [&str; 2],
        work_dir: &Path,
    ) -> Result<Archive> {
        for root in [root_a, root_b] {
            if Disk::detect(root)?.is_some() {
                return Err(Error::AlreadyInitialized(root.to_path_buf()));
            }
        }
        let archive_id = uuid::Uuid::new_v4().to_string();
        let disks = [
            Disk::init(root_a, &archive_id, Side::A, labels[0])?,
            Disk::init(root_b, &archive_id, Side::B, labels[1])?,
        ];
        let work = work_path(work_dir, &archive_id)?;
        let _ = fs::remove_file(&work);
        let mut index = Index::create(&work, &archive_id)?;
        index.update(|tx| {
            for disk in &disks {
                tx.add_disk(disk.id(), disk.label())?;
            }
            tx.log("created", None, None, None)
        })?;
        let archive = Archive {
            disks,
            index,
            info: OpenInfo::default(),
        };
        archive.save()?;
        Ok(archive)
    }

    /// Apre una coppia già preparata, in qualsiasi ordine: il lato di ogni
    /// disco è scritto nel disco. Usa l'indice più recente tra i due e lo
    /// riscrive su entrambi i dischi.
    pub fn open(root_1: &Path, root_2: &Path, work_dir: &Path) -> Result<Archive> {
        let mut disks = [Disk::open(root_1)?, Disk::open(root_2)?];
        if disks[0].side() == Side::B {
            disks.swap(0, 1);
        }
        let archive_id = disks[0].identity().archive_id.clone();
        if disks[1].identity().archive_id != archive_id {
            return Err(Error::DifferentArchives);
        }
        if disks[0].id() == disks[1].id() || disks[0].side() == disks[1].side() {
            return Err(Error::SameDisk);
        }

        let mut info = OpenInfo::default();
        for (i, disk) in disks.iter().enumerate() {
            info.revisions[i] = match Index::peek(&disk.index_path()) {
                Ok((id, rev)) if id == archive_id => Some(rev),
                _ => None,
            };
        }
        let best = match info.revisions {
            [Some(a), Some(b)] => Some(if b > a { 1 } else { 0 }),
            [Some(_), None] => Some(0),
            [None, Some(_)] => Some(1),
            [None, None] => None,
        };

        let work = work_path(work_dir, &archive_id)?;
        let mut index = match best {
            Some(i) => Index::copy_from(&disks[i].index_path(), &work)?,
            None => {
                info.index_rebuilt = true;
                let _ = fs::remove_file(&work);
                let mut index = Index::create(&work, &archive_id)?;
                index.update(|tx| tx.log("index_rebuilt", None, None, None))?;
                index
            }
        };
        index.update(|tx| {
            for disk in &disks {
                tx.add_disk(disk.id(), disk.label())?;
            }
            Ok(())
        })?;
        info.interrupted = index.journal()?;
        // La pagina si riscrive se manca, per esempio dopo una pulizia del
        // disco o un aggiornamento dell'app. È solo una comodità: un errore
        // qui non deve impedire di allineare.
        for disk in &disks {
            let _ = disk.write_download_page();
        }

        let archive = Archive { disks, index, info };
        archive.save()?;
        Ok(archive)
    }

    pub fn disk(&self, side: Side) -> &Disk {
        &self.disks[side.idx()]
    }

    pub fn disk_mut(&mut self, side: Side) -> &mut Disk {
        &mut self.disks[side.idx()]
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    pub fn index_mut(&mut self) -> &mut Index {
        &mut self.index
    }

    pub fn open_info(&self) -> &OpenInfo {
        &self.info
    }

    /// Scrive l'indice su entrambi i dischi. Se uno non risponde, prova
    /// comunque l'altro e restituisce il primo errore.
    pub fn save(&self) -> Result<()> {
        let results = self
            .disks
            .each_ref()
            .map(|d| self.index.save_to(&d.index_path()));
        results.into_iter().collect()
    }
}

fn work_path(work_dir: &Path, archive_id: &str) -> Result<PathBuf> {
    fs::create_dir_all(work_dir)?;
    Ok(work_dir.join(format!("{archive_id}.sqlite")))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Env {
        _dir: tempfile::TempDir,
        a: PathBuf,
        b: PathBuf,
        work: PathBuf,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let work = dir.path().join("work");
        Env {
            a,
            b,
            work,
            _dir: dir,
        }
    }

    #[test]
    fn create_then_open() {
        let e = env();
        let created = Archive::create(&e.a, &e.b, ["Disco A", "Disco B"], &e.work).unwrap();
        let id = created.index().archive_id().unwrap();
        drop(created);
        let archive = Archive::open(&e.a, &e.b, &e.work).unwrap();
        assert_eq!(archive.index().archive_id().unwrap(), id);
        assert_eq!(archive.disk(Side::B).label(), "Disco B");
        assert!(!archive.open_info().index_rebuilt);
    }

    #[test]
    fn sides_come_from_the_disks() {
        let e = env();
        drop(Archive::create(&e.a, &e.b, ["Rosso", "Blu"], &e.work).unwrap());
        let archive = Archive::open(&e.b, &e.a, &e.work).unwrap();
        assert_eq!(archive.disk(Side::A).root(), e.a);
        assert_eq!(archive.disk(Side::A).label(), "Rosso");
        assert_eq!(archive.disk(Side::B).side(), Side::B);
    }

    #[test]
    fn create_refuses_prepared_disk() {
        let e = env();
        Disk::init(&e.b, "x", Side::B, "B").unwrap();
        assert!(matches!(
            Archive::create(&e.a, &e.b, ["A", "B"], &e.work),
            Err(Error::AlreadyInitialized(_))
        ));
        assert!(Disk::detect(&e.a).unwrap().is_none());
    }

    #[test]
    fn different_pairs_are_refused() {
        let e = env();
        Disk::init(&e.a, "uno", Side::A, "A").unwrap();
        Disk::init(&e.b, "due", Side::B, "B").unwrap();
        assert!(matches!(
            Archive::open(&e.a, &e.b, &e.work),
            Err(Error::DifferentArchives)
        ));
    }

    #[test]
    fn same_disk_twice_is_refused() {
        let e = env();
        Archive::create(&e.a, &e.b, ["A", "B"], &e.work).unwrap();
        assert!(matches!(
            Archive::open(&e.a, &e.a, &e.work),
            Err(Error::SameDisk)
        ));
    }

    #[test]
    fn newest_index_wins_and_is_copied_to_both() {
        let e = env();
        let archive = Archive::create(&e.a, &e.b, ["A", "B"], &e.work).unwrap();
        let old_b = fs::read(archive.disk(Side::B).index_path()).unwrap();
        let mut archive = archive;
        archive
            .index_mut()
            .update(|tx| tx.log("test", None, None, None))
            .unwrap();
        archive.save().unwrap();
        // Il disco B torna all'indice vecchio, come dopo un'interruzione.
        fs::write(archive.disk(Side::B).index_path(), old_b).unwrap();
        let rev = archive.index().revision().unwrap();
        drop(archive);

        let archive = Archive::open(&e.a, &e.b, &e.work).unwrap();
        let [ra, rb] = archive.open_info().revisions;
        assert!(ra.unwrap() > rb.unwrap());
        assert!(archive.index().revision().unwrap() >= rev);
        let b_rev = Index::peek(&archive.disk(Side::B).index_path()).unwrap().1;
        assert_eq!(b_rev, archive.index().revision().unwrap());
    }

    #[test]
    fn lost_index_is_rebuilt() {
        let e = env();
        let archive = Archive::create(&e.a, &e.b, ["A", "B"], &e.work).unwrap();
        for side in Side::BOTH {
            fs::write(archive.disk(side).index_path(), b"rovinato").unwrap();
        }
        drop(archive);
        let archive = Archive::open(&e.a, &e.b, &e.work).unwrap();
        assert!(archive.open_info().index_rebuilt);
        assert!(Index::peek(&archive.disk(Side::A).index_path()).is_ok());
    }

    #[test]
    fn index_unreadable_on_one_disk() {
        let e = env();
        let archive = Archive::create(&e.a, &e.b, ["A", "B"], &e.work).unwrap();
        fs::remove_file(archive.disk(Side::A).index_path()).unwrap();
        drop(archive);
        let archive = Archive::open(&e.a, &e.b, &e.work).unwrap();
        assert!(!archive.open_info().index_rebuilt);
        assert_eq!(archive.open_info().revisions[0], None);
        assert!(archive.disk(Side::A).index_path().exists());
    }
}
