//! Dischi simulati, di default in `dev-disks/disk-a` e `dev-disks/disk-b`.
//!
//! ```sh
//! cargo run -p hddraid-devtools --bin dev-disks -- setup
//! cargo run -p hddraid-devtools --bin dev-disks -- break conflict
//! cargo run -p hddraid-devtools --bin dev-disks -- setup --a /Volumes/A --b /Volumes/B
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hddraid_devtools::{DevDisks, Scenario};

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut disks = DevDisks::in_dir(Path::new("dev-disks"));
    if let Some(i) = args.iter().position(|a| a == "--a") {
        disks.a = PathBuf::from(args.get(i + 1).cloned().unwrap_or_default());
        args.drain(i..(i + 2).min(args.len()));
    }
    if let Some(i) = args.iter().position(|a| a == "--b") {
        disks.b = PathBuf::from(args.get(i + 1).cloned().unwrap_or_default());
        args.drain(i..(i + 2).min(args.len()));
    }
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["setup"] => disks.setup(),
        ["break", name] => match Scenario::from_name(name) {
            Some(scenario) => disks.apply(scenario),
            None => return usage(),
        },
        _ => return usage(),
    };
    match result {
        Ok(()) => {
            println!("Fatto: {} e {}", disks.a.display(), disks.b.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Errore: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    let names: Vec<&str> = Scenario::NAMES.iter().map(|(n, _)| *n).collect();
    eprintln!(
        "Uso: dev-disks setup | dev-disks break <scenario>  [--a <cartella>] [--b <cartella>]"
    );
    eprintln!("Scenari: {}", names.join(", "));
    ExitCode::FAILURE
}
