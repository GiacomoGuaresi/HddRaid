//! CLI di debug del motore: serve a provarlo sui dischi simulati, prima che
//! ci sia l'interfaccia.
//!
//! ```sh
//! cargo run -p hddraid-cli -- init dev-disks/disk-a dev-disks/disk-b
//! cargo run -p hddraid-cli -- status dev-disks/disk-a dev-disks/disk-b
//! ```

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::ValueEnum;
use clap::{Parser, Subcommand};
use hddraid_core::plan::{Action, Answer, Question, QuestionKind};
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
    /// Allinea i dischi.
    Sync {
        a: PathBuf,
        b: PathBuf,
        /// Risposta da dare a tutte le domande (altrimenti le chiede).
        #[arg(long)]
        answer: Option<CliAnswer>,
        /// Non chiedere conferma prima di eseguire.
        #[arg(long)]
        yes: bool,
    },
    /// Controllo completo: rilegge tutti i file e cerca quelli rovinati.
    Check {
        a: PathBuf,
        b: PathBuf,
        /// Ripara subito i file rovinati dalla copia buona.
        #[arg(long)]
        repair: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CliAnswer {
    /// Cancella anche dall'altro disco.
    Delete,
    /// Rimetti il file dove manca.
    Restore,
    /// Tieni la versione del disco A.
    KeepA,
    /// Tieni la versione del disco B.
    KeepB,
    /// Tieni entrambe le versioni.
    Both,
    /// Decidi dopo.
    Skip,
}

impl CliAnswer {
    fn answer(self) -> Option<Answer> {
        match self {
            CliAnswer::Delete => Some(Answer::DeleteEverywhere),
            CliAnswer::Restore => Some(Answer::Restore),
            CliAnswer::KeepA => Some(Answer::Keep(Side::A)),
            CliAnswer::KeepB => Some(Answer::Keep(Side::B)),
            CliAnswer::Both => Some(Answer::KeepBoth),
            CliAnswer::Skip => None,
        }
    }
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
        Command::Sync { a, b, answer, yes } => {
            let mut archive = Archive::open(&a, &b, work)?;
            let plan = archive.plan()?;
            if plan.is_empty() {
                println!("I dischi sono già uguali.");
                archive.apply(&plan.actions, &mut |_| {})?;
                return Ok(());
            }
            let mut actions = plan.actions.clone();
            for action in &plan.actions {
                if let Some(line) = describe(action) {
                    println!("  {line}");
                }
            }
            for q in &plan.questions {
                let choice = match answer {
                    Some(a) => a,
                    None => ask(q),
                };
                if let Some(a) = choice.answer() {
                    actions.extend(archive.resolve(q, a));
                }
            }
            if !yes && !confirm("Procedo?") {
                return Ok(());
            }
            let report = archive.apply(&actions, &mut |p| {
                if !p.current.is_empty() {
                    eprint!("\r[{}/{}] {:<60.60}", p.done + 1, p.total, p.current);
                }
            })?;
            eprintln!();
            println!(
                "Fatto: {} azioni, {} byte copiati.",
                report.done, report.bytes_copied
            );
            for s in &report.skipped {
                println!("Saltato {}: {}", s.path, s.reason);
            }
        }
        Command::Check { a, b, repair } => {
            let mut archive = Archive::open(&a, &b, work)?;
            let cancel = std::sync::atomic::AtomicBool::new(false);
            let report = archive.full_check(
                &mut |p| eprint!("\r{}/{} file controllati", p.checked, p.total),
                &cancel,
            )?;
            eprintln!();
            println!("File in salute: {}", report.ok);
            for r in &report.repairs {
                println!("Rovinato sul disco {}: {}", r.bad, r.path);
            }
            for path in &report.lost {
                println!("Rovinato su entrambi i dischi: {path}");
            }
            for path in &report.changed {
                println!("Cambiato (lo sistema l'allineamento): {path}");
            }
            if repair && !report.repairs.is_empty() {
                let done = archive.repair(&report.repairs, &mut |_| {})?;
                println!("Riparati: {}", done.done);
                for s in &done.skipped {
                    println!("Non riparato {}: {}", s.path, s.reason);
                }
            }
        }
    }
    Ok(())
}

fn describe(action: &Action) -> Option<String> {
    Some(match action {
        Action::Copy { from, src, .. } => {
            format!("copia {} → {}: {}", from, from.other(), src.path)
        }
        Action::Move { side, from, to, .. } => format!("sposta su {side}: {from} → {to}"),
        Action::Delete { side, expect, .. } => format!("cancella da {side}: {}", expect.path),
        Action::SetMtime { side, entry } => format!("allinea la data su {side}: {}", entry.path),
        Action::MakeDir { side, path } => format!("crea su {side}: {path}/"),
        Action::RemoveDir { side, path } => format!("cancella da {side}: {path}/"),
        Action::Rename { side, from, to } => format!("rinomina su {side}: {from} → {to}"),
        Action::Record { .. } | Action::Forget { .. } => return None,
    })
}

fn ask(q: &Question) -> CliAnswer {
    match q.kind {
        QuestionKind::Deletion { deleted_on } => {
            println!("\n{} è stato cancellato dal disco {deleted_on}.", q.path());
            prompt(
                "[d] cancella anche dall'altro  [r] rimetti  [s] decidi dopo",
                &[
                    ("d", CliAnswer::Delete),
                    ("r", CliAnswer::Restore),
                    ("s", CliAnswer::Skip),
                ],
            )
        }
        QuestionKind::Conflict => {
            println!("\nConflitto su {}:", q.path());
            for side in Side::BOTH {
                match q.version(side) {
                    Some(v) => println!("  {side}: {} byte", v.size),
                    None => println!("  {side}: cancellato"),
                }
            }
            prompt(
                "[a] tieni A  [b] tieni B  [e] tienili entrambi  [s] decidi dopo",
                &[
                    ("a", CliAnswer::KeepA),
                    ("b", CliAnswer::KeepB),
                    ("e", CliAnswer::Both),
                    ("s", CliAnswer::Skip),
                ],
            )
        }
    }
}

fn prompt(text: &str, choices: &[(&str, CliAnswer)]) -> CliAnswer {
    loop {
        print!("{text}: ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if std::io::stdin().lock().read_line(&mut line).unwrap_or(0) == 0 {
            return CliAnswer::Skip;
        }
        if let Some((_, a)) = choices.iter().find(|(k, _)| *k == line.trim()) {
            return *a;
        }
    }
}

fn confirm(text: &str) -> bool {
    print!("{text} [s/N] ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).is_ok() && line.trim().eq_ignore_ascii_case("s")
}

fn print_disks(archive: &Archive) {
    for side in Side::BOTH {
        let disk = archive.disk(side);
        println!("{side}: {} ({})", disk.label(), disk.root().display());
    }
}
