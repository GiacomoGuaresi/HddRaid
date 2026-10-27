//! Hash SHA-256 dei file, in streaming. Lo stesso valore si ottiene senza
//! l'app con `shasum -a 256` o `certutil -hashfile <file> SHA256`.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

/// Dimensione dei blocchi letti dal disco.
pub const BUF_SIZE: usize = 1024 * 1024;

/// Hash incrementale.
#[derive(Default)]
pub struct Hasher(Sha256);

impl Hasher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    /// Hash in esadecimale minuscolo.
    pub fn finish(self) -> String {
        to_hex(&self.0.finalize())
    }
}

/// Hash di un file.
pub fn hash_file(path: &Path) -> io::Result<String> {
    hash_reader(File::open(path)?)
}

/// Hash di tutto quello che si legge da `reader`.
pub fn hash_reader(mut reader: impl Read) -> io::Result<String> {
    let mut hasher = Hasher::new();
    let mut buf = vec![0u8; BUF_SIZE];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            return Ok(hasher.finish());
        }
        hasher.update(&buf[..n]);
    }
}

fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0xF) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values() {
        assert_eq!(
            hash_reader(&b""[..]).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash_reader(&b"abc"[..]).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn file_larger_than_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.bin");
        let data: Vec<u8> = (0..BUF_SIZE * 2 + 7).map(|i| i as u8).collect();
        std::fs::write(&path, &data).unwrap();
        assert_eq!(hash_file(&path).unwrap(), hash_reader(&data[..]).unwrap());
    }
}
