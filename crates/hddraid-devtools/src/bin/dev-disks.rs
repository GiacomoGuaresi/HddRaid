//! Dischi simulati in `dev-disks/`.
//!
//! ```sh
//! cargo run -p hddraid-devtools --bin dev-disks -- setup
//! cargo run -p hddraid-devtools --bin dev-disks -- break conflict
//! ```

use std::path::Path;
use std::process::ExitCode;

use hddraid_devtools::{DevDisks, Scenario};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let disks = DevDisks::in_dir(Path::new("dev-disks"));
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
    eprintln!("Uso: dev-disks setup | dev-disks break <scenario>");
    eprintln!("Scenari: {}", names.join(", "));
    ExitCode::FAILURE
}
