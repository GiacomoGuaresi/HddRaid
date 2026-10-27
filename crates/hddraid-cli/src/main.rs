//! CLI di debug del motore: serve a provarlo sui dischi simulati, prima che
//! ci sia l'interfaccia.
//!
//! ```sh
//! cargo run -p hddraid-cli -- init dev-disks/disk-a dev-disks/disk-b
//! cargo run -p hddraid-cli -- status dev-disks/disk-a dev-disks/disk-b
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use hddraid_core::{Archive, Side, scan};

#[derive(Parser)]
#[command(name = "hddraid", version, about = "CLI di debug di HddRaid")]
struct Cli {
    /// Cartella per la copia locale dell'indice.
    #[arg(long, global = true, env = "HDDRAID_WORK")]
    work: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Prepara due dischi come una nuova coppia.
    Init {
        a: PathBuf,
        b: PathBuf,
        #[arg(long, default_value = "Disco A")]
        label_a: String,
        #[arg(long, default_value = "Disco B")]
        label_b: String,
    },
    /// Elenca i file di un disco come li vede il motore.
    Scan { dir: PathBuf },
    /// Mostra lo stato della coppia.
    Status { a: PathBuf, b: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let work = cli
        .work
        .unwrap_or_else(|| std::env::temp_dir().join("hddraid-work"));
    match run(cli.command, &work) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Errore: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command, work: &Path) -> hddraid_core::Result<()> {
    match command {
        Command::Init {
            a,
            b,
            label_a,
            label_b,
        } => {
            let archive = Archive::create(&a, &b, [&label_a, &label_b], work)?;
            println!("Coppia creata: {}", archive.index().archive_id()?);
            print_disks(&archive);
        }
        Command::Scan { dir } => {
            let scan = scan::scan(&dir)?;
            for file in scan.files.values() {
                println!("{:>12}  {}", file.size, file.path);
            }
            for d in scan.dirs.values().filter(|d| d.empty) {
                println!("{:>12}  {}/", "(vuota)", d.path);
            }
            for s in &scan.skipped {
                println!("{:>12}  {s}", "(saltato)");
            }
            println!(
                "{} file, {} cartelle vuote",
                scan.files.len(),
                scan.dirs.values().filter(|d| d.empty).count()
            );
        }
        Command::Status { a, b } => {
            let archive = Archive::open(&a, &b, work)?;
            let info = archive.open_info();
            print_disks(&archive);
            println!("Revisione indice: {}", archive.index().revision()?);
            if info.index_rebuilt {
                println!("Indice ricreato: il prossimo allineamento unirà i dischi.");
            }
            for op in &info.interrupted {
                println!("Operazione interrotta: {} {} ({})", op.op, op.params, op.at);
            }
            println!("File nella base: {}", archive.index().base()?.len());
        }
    }
    Ok(())
}

fn print_disks(archive: &Archive) {
    for side in Side::BOTH {
        let disk = archive.disk(side);
        println!("{side}: {} ({})", disk.label(), disk.root().display());
    }
}
