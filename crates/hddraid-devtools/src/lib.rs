//! Dischi simulati per sviluppo e test: due cartelle che fanno da disco A e
//! disco B, con un contenuto finto ma realistico, e scenari che le modificano
//! come farebbe l'utente (o un guasto).
//!
//! Tutto è deterministico: stesso seme, stessi file.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Le due cartelle che simulano i dischi.
#[derive(Debug, Clone)]
pub struct DevDisks {
    pub a: PathBuf,
    pub b: PathBuf,
}

impl DevDisks {
    /// Dischi dentro `root`: `root/disk-a` e `root/disk-b`.
    pub fn in_dir(root: &Path) -> Self {
        Self {
            a: root.join("disk-a"),
            b: root.join("disk-b"),
        }
    }

    /// Ricrea i due dischi da zero, con lo stesso contenuto. Se una cartella
    /// esiste già (anche un volume montato) ne svuota il contenuto; le
    /// cartelle di sistema che non si possono togliere restano.
    pub fn setup(&self) -> io::Result<()> {
        for disk in [&self.a, &self.b] {
            fs::create_dir_all(disk)?;
            for entry in fs::read_dir(disk)? {
                let path = entry?.path();
                let _ = if path.is_dir() {
                    fs::remove_dir_all(&path)
                } else {
                    fs::remove_file(&path)
                };
            }
            populate(disk)?;
        }
        Ok(())
    }

    /// Applica uno scenario ai dischi.
    pub fn apply(&self, scenario: Scenario) -> io::Result<()> {
        let (a, b) = (&self.a, &self.b);
        match scenario {
            Scenario::New => {
                let dir = a.join("Foto/2026/Natale 2026");
                fs::create_dir_all(&dir)?;
                let mut rng = Rng::new(2026);
                for i in 1..=5 {
                    write_fake(&dir.join(format!("IMG_{i:04}.jpg")), &mut rng, 40_000)?;
                }
            }
            Scenario::Modify => {
                append(
                    &a.join("Documenti/Note.txt"),
                    b"Riga aggiunta sul disco A.\n",
                )?;
            }
            Scenario::Delete => {
                fs::remove_file(b.join("Foto/2024/Mare 2024/IMG_0002.jpg"))?;
            }
            Scenario::Move => {
                fs::rename(
                    a.join("Foto/2023/Montagna"),
                    a.join("Foto/2023/Dolomiti 2023"),
                )?;
            }
            Scenario::Conflict => {
                append(
                    &a.join("Documenti/Lista.txt"),
                    b"Modifica fatta sul disco A.\n",
                )?;
                append(
                    &b.join("Documenti/Lista.txt"),
                    b"Modifica diversa sul disco B.\n",
                )?;
            }
            Scenario::SameContent => {
                // Stesso file nuovo su entrambi, con date diverse: non è un conflitto.
                for (disk, age) in [(a, 0), (b, 3600)] {
                    let path = disk.join("Documenti/Ricevuta.pdf");
                    write_fake(&path, &mut Rng::new(42), 30_000)?;
                    set_mtime(&path, SystemTime::now() - Duration::from_secs(age))?;
                }
            }
            Scenario::Corrupt => {
                // Un byte cambiato, stessa dimensione e stessa data: bit rot.
                flip_byte(&b.join("Foto/2025/Compleanno/IMG_0001.jpg"))?;
            }
            Scenario::EmptyDir => {
                fs::create_dir_all(a.join("Foto/2027"))?;
            }
            Scenario::Case => {
                // In due passi, così cambia davvero anche su file system che
                // non distinguono maiuscole e minuscole.
                let dir = a.join("Documenti");
                fs::rename(dir.join("Lista.txt"), dir.join("lista.tmp"))?;
                fs::rename(dir.join("lista.tmp"), dir.join("LISTA.txt"))?;
            }
            Scenario::System => {
                fs::write(a.join(".DS_Store"), b"finto")?;
                fs::write(a.join("Foto/Thumbs.db"), b"finto")?;
                fs::create_dir_all(b.join("$RECYCLE.BIN"))?;
                fs::write(b.join("$RECYCLE.BIN/desktop.ini"), b"finto")?;
                fs::write(a.join("Foto/2024/._IMG_0001.jpg"), b"finto")?;
            }
            Scenario::All => {
                for s in Scenario::ALL_SINGLE {
                    self.apply(s)?;
                }
            }
        }
        Ok(())
    }
}

/// Modifiche ai dischi simulati.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// File nuovi sul disco A.
    New,
    /// File modificato sul disco A.
    Modify,
    /// File cancellato dal disco B.
    Delete,
    /// Cartella rinominata sul disco A.
    Move,
    /// Stesso file modificato in modo diverso su A e B.
    Conflict,
    /// Stesso file nuovo su A e B, con date diverse.
    SameContent,
    /// File rovinato sul disco B (stessa dimensione e data).
    Corrupt,
    /// Cartella vuota sul disco A.
    EmptyDir,
    /// File rinominato cambiando solo le maiuscole sul disco A.
    Case,
    /// File di sistema da ignorare su entrambi.
    System,
    /// Tutti gli scenari insieme.
    All,
}

impl Scenario {
    const ALL_SINGLE: [Scenario; 10] = [
        Scenario::New,
        Scenario::Modify,
        Scenario::Delete,
        Scenario::Move,
        Scenario::Conflict,
        Scenario::SameContent,
        Scenario::Corrupt,
        Scenario::EmptyDir,
        Scenario::Case,
        Scenario::System,
    ];

    pub const NAMES: [(&'static str, Scenario); 11] = [
        ("new", Scenario::New),
        ("modify", Scenario::Modify),
        ("delete", Scenario::Delete),
        ("move", Scenario::Move),
        ("conflict", Scenario::Conflict),
        ("same-content", Scenario::SameContent),
        ("corrupt", Scenario::Corrupt),
        ("empty-dir", Scenario::EmptyDir),
        ("case", Scenario::Case),
        ("system", Scenario::System),
        ("all", Scenario::All),
    ];

    pub fn from_name(name: &str) -> Option<Self> {
        Self::NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, s)| *s)
    }
}

/// Contenuto iniziale di un disco: foto per anno ed evento, documenti, nomi
/// con accenti, un file vuoto e una cartella vuota.
fn populate(disk: &Path) -> io::Result<()> {
    let mut rng = Rng::new(1);
    let events = [
        ("Foto/2023/Montagna", 6),
        ("Foto/2024/Mare 2024", 8),
        ("Foto/2024/Perché no", 3),
        ("Foto/2025/Compleanno", 5),
    ];
    for (dir, count) in events {
        let dir = disk.join(dir);
        fs::create_dir_all(&dir)?;
        for i in 1..=count {
            write_fake(&dir.join(format!("IMG_{i:04}.jpg")), &mut rng, 80_000)?;
        }
    }
    let docs = disk.join("Documenti");
    fs::create_dir_all(&docs)?;
    fs::write(docs.join("Note.txt"), "Note importanti.\n")?;
    fs::write(docs.join("Lista.txt"), "Cose da fare.\n")?;
    write_fake(&docs.join("Contratto affitto.pdf"), &mut rng, 50_000)?;
    File::create(docs.join("Vuoto.txt"))?;
    fs::create_dir_all(disk.join("Foto/2026"))?;
    Ok(())
}

/// Scrive un file di byte pseudo-casuali, tra metà e tutta la dimensione
/// indicata. Le foto iniziano con l'intestazione JPEG.
fn write_fake(path: &Path, rng: &mut Rng, max_size: usize) -> io::Result<()> {
    let size = max_size / 2 + rng.below(max_size / 2);
    let mut data = Vec::with_capacity(size);
    if path.extension().is_some_and(|e| e == "jpg") {
        data.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]);
    }
    while data.len() < size {
        data.extend_from_slice(&rng.next().to_le_bytes());
    }
    data.truncate(size);
    fs::write(path, data)
}

fn append(path: &Path, text: &[u8]) -> io::Result<()> {
    OpenOptions::new().append(true).open(path)?.write_all(text)
}

fn flip_byte(path: &Path) -> io::Result<()> {
    let mtime = fs::metadata(path)?.modified()?;
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let pos = file.metadata()?.len() / 2;
    let mut byte = [0u8];
    file.seek(SeekFrom::Start(pos))?;
    file.read_exact(&mut byte)?;
    file.seek(SeekFrom::Start(pos))?;
    file.write_all(&[byte[0] ^ 0xFF])?;
    file.set_modified(mtime)
}

fn set_mtime(path: &Path, time: SystemTime) -> io::Result<()> {
    OpenOptions::new()
        .write(true)
        .open(path)?
        .set_modified(time)
}

/// Generatore pseudo-casuale xorshift64: basta per dati finti riproducibili.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("hddraid-devtools-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        root
    }

    fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for entry in fs::read_dir(&d).unwrap() {
                let path = entry.unwrap().path();
                let rel = path
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                if path.is_dir() {
                    out.push((rel + "/", Vec::new()));
                    stack.push(path);
                } else {
                    out.push((rel, fs::read(&path).unwrap()));
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn setup_makes_identical_disks() {
        let root = temp_root("setup");
        let disks = DevDisks::in_dir(&root);
        disks.setup().unwrap();
        assert_eq!(snapshot(&disks.a), snapshot(&disks.b));
        assert!(
            disks
                .a
                .join("Foto/2026")
                .read_dir()
                .unwrap()
                .next()
                .is_none()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn all_scenarios_apply() {
        let root = temp_root("all");
        let disks = DevDisks::in_dir(&root);
        disks.setup().unwrap();
        disks.apply(Scenario::All).unwrap();
        assert_ne!(snapshot(&disks.a), snapshot(&disks.b));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_keeps_size_and_mtime() {
        let root = temp_root("corrupt");
        let disks = DevDisks::in_dir(&root);
        disks.setup().unwrap();
        let path = disks.b.join("Foto/2025/Compleanno/IMG_0001.jpg");
        let before = fs::metadata(&path).unwrap();
        let content = fs::read(&path).unwrap();
        disks.apply(Scenario::Corrupt).unwrap();
        let after = fs::metadata(&path).unwrap();
        assert_eq!(before.len(), after.len());
        assert_eq!(before.modified().unwrap(), after.modified().unwrap());
        assert_ne!(content, fs::read(&path).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scenario_names_round_trip() {
        for (name, scenario) in Scenario::NAMES {
            assert_eq!(Scenario::from_name(name), Some(scenario));
        }
        assert_eq!(Scenario::from_name("boh"), None);
    }
}
