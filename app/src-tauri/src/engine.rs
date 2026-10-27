//! Il lavoro in background: un solo thread che rileva i dischi, li confronta,
//! li allinea e li controlla. I comandi dell'interfaccia arrivano come
//! messaggi; lo stato si pubblica con l'evento `state`.
//!
//! Un solo thread vuol dire anche una sola [`Archive`] aperta alla volta: la
//! copia di lavoro dell'indice non viene mai aperta due volte.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use hddraid_core::check::Repair;
use hddraid_core::index::{Index, Kind};
use hddraid_core::plan::Plan;
use hddraid_core::{Archive, Disk, Error, Side, volumes};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::settings::{Config, KnownDisk, Pair};
use crate::texts::{Lang, Texts};
use crate::view::{
    self, AnswerView, Busy, DiskView, ErrorCode, HistoryEvent, Outcome, PairView, ProgressView,
    RepairView, SkippedView, State,
};

/// Ogni quanto guardare i volumi collegati.
const POLL: Duration = Duration::from_secs(2);
/// Ogni quanto, al massimo, pubblicare l'avanzamento.
const PROGRESS_EVERY: Duration = Duration::from_millis(150);
/// Un nuovo confronto su richiesta non prima di questo intervallo.
const REFRESH_MIN: Duration = Duration::from_secs(5);
/// "Più tardi" sul controllo completo.
const POSTPONE: chrono::Duration = chrono::Duration::days(7);
/// Velocità di lettura stimata di un disco esterno, per il controllo completo.
const READ_SPEED: u64 = 100_000_000;

pub enum Cmd {
    /// Confronta di nuovo i dischi, se collegati e liberi.
    Refresh,
    Sync(BTreeMap<String, AnswerView>),
    Eject,
    StartCheck,
    PostponeCheck,
    Repair,
    Dismiss,
    Rename(Side, String),
    /// Le impostazioni sono cambiate (lingua, intervallo dei controlli).
    Settings,
}

/// Ciò che i comandi Tauri condividono con il thread.
pub struct Shared {
    pub state: Mutex<State>,
    pub config: Mutex<Config>,
    pub config_dir: PathBuf,
    pub work_dir: PathBuf,
    /// Ferma il controllo completo in corso.
    pub cancel: AtomicBool,
    tx: Mutex<Sender<Cmd>>,
}

impl Shared {
    pub fn send(&self, cmd: Cmd) {
        let _ = self.tx.lock().unwrap().send(cmd);
    }

    pub fn texts(&self) -> Texts {
        Texts::new(Lang::pick(
            self.config.lock().unwrap().settings.language.as_deref(),
        ))
    }

    /// Storico della coppia, anche a dischi scollegati: si legge dalla copia
    /// locale dell'indice.
    pub fn history(&self, limit: usize) -> Vec<HistoryEvent> {
        let Some(pair) = self.config.lock().unwrap().pair.clone() else {
            return Vec::new();
        };
        let path = self.work_dir.join(format!("{}.sqlite", pair.archive_id));
        let events = Index::open(&path).and_then(|i| i.events(limit));
        match events {
            Ok(events) => events
                .into_iter()
                .map(|e| HistoryEvent {
                    disk: e
                        .disk_id
                        .as_deref()
                        .and_then(|id| pair.label_of(id))
                        .map(str::to_string),
                    at: e.at,
                    kind: e.kind,
                    path: e.path,
                    details: e.details,
                })
                .collect(),
            Err(e) => {
                log::warn!("storico non leggibile: {e}");
                Vec::new()
            }
        }
    }
}

/// Avvia il thread e restituisce lo stato condiviso.
pub fn start(app: AppHandle, config_dir: PathBuf, work_dir: PathBuf) -> Arc<Shared> {
    let (tx, rx) = mpsc::channel();
    let config = Config::load(&config_dir);
    let state = State {
        pair: config.pair.as_ref().map(pair_view),
        ..State::default()
    };
    let shared = Arc::new(Shared {
        state: Mutex::new(state),
        config: Mutex::new(config),
        config_dir,
        work_dir,
        cancel: AtomicBool::new(false),
        tx: Mutex::new(tx),
    });
    let engine = Engine {
        app,
        shared: shared.clone(),
        archive: None,
        plan: None,
        repairs: Vec::new(),
        present: [None, None],
        ejected: HashSet::new(),
        last_compare: None,
    };
    std::thread::Builder::new()
        .name("engine".into())
        .spawn(move || engine.run(rx))
        .expect("thread del motore");
    shared
}

fn pair_view(pair: &Pair) -> PairView {
    PairView {
        labels: Side::BOTH
            .map(|s| (s, pair.label(s).to_string()))
            .into_iter()
            .collect(),
    }
}

struct Engine {
    app: AppHandle,
    shared: Arc<Shared>,
    archive: Option<Archive>,
    plan: Option<Plan>,
    repairs: Vec<Repair>,
    /// Identificativi dei dischi della coppia collegati, per lato.
    present: [Option<String>; 2],
    /// Volumi espulsi: si ignorano finché restano montati.
    ejected: HashSet<PathBuf>,
    last_compare: Option<Instant>,
}

impl Engine {
    fn run(mut self, rx: Receiver<Cmd>) {
        self.poll();
        loop {
            match rx.recv_timeout(POLL) {
                Ok(cmd) => self.handle(cmd),
                Err(RecvTimeoutError::Timeout) => self.poll(),
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Refresh => {
                let recent = self.last_compare.is_some_and(|t| t.elapsed() < REFRESH_MIN);
                if self.archive.is_some() && !self.busy() && !recent {
                    self.compare(false);
                }
            }
            Cmd::Sync(answers) => self.sync(answers),
            Cmd::Eject => self.eject(),
            Cmd::StartCheck => self.check(),
            Cmd::PostponeCheck => {
                let until = (Utc::now() + POSTPONE).to_rfc3339();
                self.update_config(|c| c.check_postponed_until = Some(until));
                self.set(|s| s.check_due = false);
            }
            Cmd::Repair => self.repair(),
            Cmd::Dismiss => self.set(|s| s.outcome = None),
            Cmd::Rename(side, label) => self.rename(side, &label),
            Cmd::Settings => {
                let due = self.check_due();
                self.set(|s| s.check_due = due);
            }
        }
    }

    fn busy(&self) -> bool {
        self.shared.state.lock().unwrap().busy.is_some()
    }

    /// Modifica lo stato e lo pubblica, se è cambiato.
    fn set(&self, f: impl FnOnce(&mut State)) {
        let snapshot = {
            let mut state = self.shared.state.lock().unwrap();
            let before = state.clone();
            f(&mut state);
            if *state == before {
                return;
            }
            state.clone()
        };
        let _ = self.app.emit("state", &snapshot);
        crate::tray::update(&self.app, &snapshot, &self.shared.texts());
    }

    fn update_config(&self, f: impl FnOnce(&mut Config)) {
        let mut config = self.shared.config.lock().unwrap();
        f(&mut config);
        if let Err(e) = config.save(&self.shared.config_dir) {
            log::error!("impostazioni non salvate: {e}");
        }
    }

    fn notify(&self, (title, body): (String, String)) {
        if let Err(e) = self
            .app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
        {
            log::warn!("notifica non mostrata: {e}");
        }
    }

    // --- Rilevamento -------------------------------------------------------

    fn poll(&mut self) {
        if self.busy() {
            // Durante un'operazione è il motore ad accorgersi dei dischi
            // scollegati.
            return;
        }
        let roots = volumes::mounted();
        self.ejected.retain(|r| roots.contains(r));
        let disks: Vec<Disk> = roots
            .iter()
            .filter(|r| !self.ejected.contains(*r))
            .filter_map(|r| match Disk::detect(r) {
                Ok(d) => d,
                Err(e) => {
                    log::debug!("{}: {e}", r.display());
                    None
                }
            })
            .collect();

        let pair = self.adopt_pair(&disks);
        let mut found: [Option<&Disk>; 2] = [None, None];
        let mut foreign = Vec::new();
        for disk in &disks {
            match &pair {
                Some(p) if disk.identity().archive_id == p.archive_id => {
                    found[disk.side() as usize] = Some(disk);
                }
                _ => foreign.push(volume_name(disk)),
            }
        }
        foreign.sort();

        let ids = found.map(|d| d.map(|d| d.id().to_string()));
        let changed = ids != self.present;
        let previous = std::mem::replace(&mut self.present, ids);
        let views = found.map(|d| d.map(disk_view));
        self.set(|s| {
            s.foreign = foreign;
            if changed {
                s.disks = Side::BOTH
                    .map(|side| (side, views[side as usize].clone()))
                    .into_iter()
                    .collect();
                s.summary = None;
                // Un esito vecchio non ha senso con dischi appena collegati;
                // a dischi scollegati resta ("Puoi scollegare i dischi").
                if found.iter().any(Option::is_some) {
                    s.outcome = None;
                }
            }
        });
        if !changed {
            return;
        }

        self.archive = None;
        self.plan = None;
        self.repairs.clear();
        match found {
            [Some(_), Some(_)] => self.compare(true),
            [Some(d), None] | [None, Some(d)] if previous.iter().all(Option::is_none) => {
                let p = pair.as_ref().unwrap();
                let missing = p.label(d.side().other()).to_string();
                self.notify(self.shared.texts().one_connected(d.label(), &missing));
            }
            _ => {}
        }
    }

    /// La coppia di questo computer. La prima volta che si vedono collegati
    /// i due dischi di una coppia, la si ricorda.
    fn adopt_pair(&self, disks: &[Disk]) -> Option<Pair> {
        if let Some(pair) = self.shared.config.lock().unwrap().pair.clone() {
            return Some(pair);
        }
        let a = disks.iter().find(|d| d.side() == Side::A)?;
        let b = disks
            .iter()
            .find(|d| d.side() == Side::B && d.identity().archive_id == a.identity().archive_id)?;
        let pair = Pair {
            archive_id: a.identity().archive_id.clone(),
            disks: [a, b].map(|d| KnownDisk {
                id: d.id().to_string(),
                label: d.label().to_string(),
            }),
        };
        log::info!("nuova coppia: {}", pair.archive_id);
        self.update_config(|c| c.pair = Some(pair.clone()));
        let view = pair_view(&pair);
        self.set(|s| s.pair = Some(view));
        Some(pair)
    }

    // --- Confronto ---------------------------------------------------------

    /// Apre la coppia (se serve) e calcola cosa c'è da fare. `connected`:
    /// i dischi sono appena stati collegati, quindi si avvisa l'utente.
    fn compare(&mut self, connected: bool) {
        self.last_compare = Some(Instant::now());
        self.set(|s| s.busy = Some(Busy::Comparing));
        match self.try_compare() {
            Ok(()) => {}
            Err(e) => {
                log::error!("confronto non riuscito: {e}");
                self.archive = None;
                let outcome = self.error(&e);
                self.set(|s| {
                    s.busy = None;
                    s.outcome = Some(outcome);
                });
                return;
            }
        }
        let state = self.shared.state.lock().unwrap().clone();
        if !connected {
            return;
        }
        let texts = self.shared.texts();
        match &state.summary {
            Some(summary) => {
                self.notify(texts.ready(summary.changes(), summary.questions.len()));
                crate::show_window(&self.app);
            }
            None if state.check_due => {
                self.notify(texts.check_due());
                crate::show_window(&self.app);
            }
            None => self.notify(texts.already_same()),
        }
    }

    fn try_compare(&mut self) -> hddraid_core::Result<()> {
        if self.archive.is_none() {
            let roots = self.roots().ok_or(Error::DifferentArchives)?;
            let archive = Archive::open(&roots[0], &roots[1], &self.shared.work_dir)?;
            if !archive.open_info().interrupted.is_empty() {
                // Rifare il piano sistema da solo le operazioni interrotte.
                log::info!(
                    "operazioni interrotte: {}",
                    archive.open_info().interrupted.len()
                );
            }
            self.archive = Some(archive);
        }
        let archive = self.archive.as_ref().unwrap();
        let plan = archive.plan()?;
        let base = archive.index().base()?;
        let free = Side::BOTH.map(|s| {
            volumes::space(archive.disk(s).root())
                .map(|(_, free)| free)
                .unwrap_or(u64::MAX)
        });
        let summary = view::summary(&plan, &base, free);
        let total_bytes: u64 = base
            .values()
            .filter(|e| e.kind == Kind::File)
            .map(|e| e.size)
            .sum();
        let last_sync = archive.index().meta("last_sync")?;
        let last_check = archive.index().meta("last_full_check")?;
        let views = Side::BOTH.map(|s| disk_view(archive.disk(s)));
        self.plan = Some(plan);
        let due = self.check_due();
        self.set(|s| {
            s.busy = None;
            s.disks = Side::BOTH
                .map(|side| (side, Some(views[side as usize].clone())))
                .into_iter()
                .collect();
            s.summary = (!summary.is_empty()).then_some(summary);
            s.last_sync = last_sync;
            s.last_check = last_check;
            s.check_due = due;
            s.check_estimate = Some(total_bytes / READ_SPEED);
        });
        Ok(())
    }

    /// Radici dei due dischi della coppia, se collegati.
    fn roots(&self) -> Option<[PathBuf; 2]> {
        let state = self.shared.state.lock().unwrap();
        let a = state.disks.get(&Side::A)?.as_ref()?;
        let b = state.disks.get(&Side::B)?.as_ref()?;
        Some([PathBuf::from(&a.root), PathBuf::from(&b.root)])
    }

    fn check_due(&self) -> bool {
        let config = self.shared.config.lock().unwrap();
        let postponed = config
            .check_postponed_until
            .as_deref()
            .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
            .is_some_and(|d| d > Utc::now());
        let every = Duration::from_secs(u64::from(config.settings.check_months) * 30 * 86_400);
        drop(config);
        !postponed
            && self
                .archive
                .as_ref()
                .is_some_and(|a| a.check_due(every).unwrap_or(false))
    }

    // --- Allineamento ------------------------------------------------------

    fn sync(&mut self, answers: BTreeMap<String, AnswerView>) {
        let (Some(archive), Some(plan)) = (self.archive.as_ref(), self.plan.take()) else {
            return;
        };
        let mut actions = plan.actions;
        for question in &plan.questions {
            if let Some(answer) = answers.get(&question.key).and_then(|a| a.to_core()) {
                actions.extend(archive.resolve(question, answer));
            }
        }

        let mut archive = self.archive.take().unwrap();
        let started = Instant::now();
        let mut last = Instant::now() - PROGRESS_EVERY;
        let mut publish = |p: &hddraid_core::sync::Progress| {
            if last.elapsed() < PROGRESS_EVERY && p.done < p.total {
                return;
            }
            last = Instant::now();
            let progress = progress_view(
                p.done,
                p.total,
                p.bytes_done,
                p.bytes_total,
                p.current,
                started,
            );
            self.set(|s| s.busy = Some(Busy::Syncing { progress }));
        };
        publish(&hddraid_core::sync::Progress {
            done: 0,
            total: actions.len(),
            bytes_done: 0,
            bytes_total: 0,
            current: "",
        });
        let result = archive.apply(&actions, &mut publish);
        self.archive = Some(archive);
        match result {
            Ok(report) => {
                log::info!(
                    "allineati: {} copiati, {} cancellati, {} saltati",
                    report.copied,
                    report.deleted,
                    report.skipped.len()
                );
                self.set(|s| {
                    s.busy = None;
                    s.outcome = Some(Outcome::Synced {
                        copied: report.copied,
                        bytes: report.bytes_copied,
                        deleted: report.deleted,
                        skipped: skipped(&report.skipped),
                    });
                });
                // Le domande rimandate e i file saltati restano da fare.
                if let Err(e) = self.try_compare() {
                    log::warn!("nuovo confronto non riuscito: {e}");
                }
                self.notify(self.shared.texts().synced());
            }
            Err(e) => {
                log::error!("allineamento interrotto: {e}");
                let outcome = self.error(&e);
                self.set(|s| {
                    s.busy = None;
                    s.outcome = Some(outcome);
                });
            }
        }
    }

    // --- Controllo completo ------------------------------------------------

    fn check(&mut self) {
        if self.archive.is_none() {
            return;
        }
        self.shared.cancel.store(false, Ordering::Relaxed);
        let started = Instant::now();
        let mut last = Instant::now() - PROGRESS_EVERY;
        let shared = self.shared.clone();
        let mut archive = self.archive.take().unwrap();
        let result = archive.full_check(
            &mut |p| {
                if last.elapsed() < PROGRESS_EVERY && p.checked < p.total {
                    return;
                }
                last = Instant::now();
                let progress =
                    progress_view(p.checked, p.total, p.bytes_done, p.bytes_total, "", started);
                self.set(|s| s.busy = Some(Busy::Checking { progress }));
            },
            &shared.cancel,
        );
        self.archive = Some(archive);
        match result {
            Ok(report) => {
                self.repairs = report.repairs.clone();
                let last_check = self
                    .archive
                    .as_ref()
                    .and_then(|a| a.index().meta("last_full_check").ok().flatten());
                self.update_config(|c| c.check_postponed_until = None);
                let due = self.check_due();
                self.set(|s| {
                    s.busy = None;
                    s.last_check = last_check;
                    s.check_due = due && !report.completed;
                    s.outcome = Some(Outcome::Checked {
                        ok: report.ok,
                        repairs: report.repairs.iter().map(RepairView::from).collect(),
                        lost: report.lost.clone(),
                        completed: report.completed,
                    });
                });
                if report.completed {
                    let texts = self.shared.texts();
                    self.notify((texts.checked(), String::new()));
                }
            }
            Err(e) => {
                log::error!("controllo interrotto: {e}");
                let outcome = self.error(&e);
                self.set(|s| {
                    s.busy = None;
                    s.outcome = Some(outcome);
                });
            }
        }
    }

    fn repair(&mut self) {
        let repairs = std::mem::take(&mut self.repairs);
        let Some(mut archive) = self.archive.take() else {
            return;
        };
        let started = Instant::now();
        let result = archive.repair(&repairs, &mut |p| {
            let progress = progress_view(p.done, p.total, 0, 0, p.current, started);
            self.set(|s| s.busy = Some(Busy::Repairing { progress }));
        });
        self.archive = Some(archive);
        let outcome = match result {
            Ok(report) => Outcome::Repaired {
                done: report.done,
                skipped: skipped(&report.skipped),
            },
            Err(e) => self.error(&e),
        };
        self.set(|s| {
            s.busy = None;
            s.outcome = Some(outcome);
        });
    }

    // --- Espulsione e nomi -------------------------------------------------

    fn eject(&mut self) {
        let Some(roots) = self.roots() else {
            return;
        };
        self.set(|s| s.busy = Some(Busy::Ejecting));
        // L'indice deve essere chiuso prima di espellere.
        self.archive = None;
        self.plan = None;
        for (i, root) in roots.iter().enumerate() {
            if let Err(e) = volumes::eject(root) {
                log::warn!("espulsione di {} non riuscita: {e}", root.display());
                let side = Side::BOTH[i];
                self.set(|s| {
                    s.busy = None;
                    s.outcome = Some(Outcome::Error {
                        code: ErrorCode::Eject,
                        disk: Some(side),
                        message: e.to_string(),
                    });
                });
                // Si riparte da capo al prossimo controllo.
                self.present = [None, None];
                return;
            }
            self.ejected.insert(root.clone());
        }
        self.present = [None, None];
        self.set(|s| {
            s.busy = None;
            s.disks = Side::BOTH.map(|side| (side, None)).into_iter().collect();
            s.summary = None;
            s.outcome = Some(Outcome::Ejected);
        });
        self.notify(self.shared.texts().ejected());
    }

    fn rename(&mut self, side: Side, label: &str) {
        let label = label.trim();
        if label.is_empty() {
            return;
        }
        let result = match self.archive.as_mut() {
            Some(archive) => archive.disk_mut(side).set_label(label).and_then(|()| {
                let id = archive.disk(side).id().to_string();
                archive.index_mut().update(|tx| tx.add_disk(&id, label))?;
                archive.save()
            }),
            None => Ok(()),
        };
        if let Err(e) = result {
            log::error!("nome non cambiato: {e}");
            let outcome = self.error(&e);
            self.set(|s| s.outcome = Some(outcome));
            return;
        }
        self.update_config(|c| {
            if let Some(pair) = c.pair.as_mut() {
                pair.disks[side as usize].label = label.to_string();
            }
        });
        let label = label.to_string();
        self.set(|s| {
            if let Some(pair) = s.pair.as_mut() {
                pair.labels.insert(side, label.clone());
            }
            if let Some(Some(disk)) = s.disks.get_mut(&side) {
                disk.label = label;
            }
        });
    }

    /// Un errore, con il motivo più probabile per l'utente.
    fn error(&self, e: &Error) -> Outcome {
        let gone = self.roots().and_then(|roots| {
            Side::BOTH.into_iter().find(|s| {
                !roots[*s as usize]
                    .join(hddraid_core::disk::DATA_DIR)
                    .is_dir()
            })
        });
        let (code, disk) = match (e, gone) {
            (_, Some(side)) => (ErrorCode::Disconnected, Some(side)),
            (Error::NoSpace { label, .. }, None) => (ErrorCode::NoSpace, self.side_of(label)),
            (Error::Io(io), None) if io.kind() == std::io::ErrorKind::PermissionDenied => {
                (ErrorCode::ReadOnly, None)
            }
            (Error::Io(io), None) if io.raw_os_error() == Some(30) => (ErrorCode::ReadOnly, None),
            _ => (ErrorCode::Other, None),
        };
        Outcome::Error {
            code,
            disk,
            message: e.to_string(),
        }
    }

    fn side_of(&self, label: &str) -> Option<Side> {
        let state = self.shared.state.lock().unwrap();
        let pair = state.pair.as_ref()?;
        pair.labels
            .iter()
            .find(|(_, l)| l.as_str() == label)
            .map(|(s, _)| *s)
    }
}

fn disk_view(disk: &Disk) -> DiskView {
    let (total, free) = volumes::space(disk.root()).unwrap_or((0, 0));
    DiskView {
        side: disk.side(),
        label: disk.label().to_string(),
        root: disk.root().to_string_lossy().into_owned(),
        total,
        free,
    }
}

/// Il nome del volume, come lo mostra Esplora file / Finder.
fn volume_name(disk: &Disk) -> String {
    disk.root()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| disk.root().to_string_lossy().into_owned())
}

fn skipped(list: &[hddraid_core::sync::Skipped]) -> Vec<SkippedView> {
    list.iter()
        .map(|s| SkippedView {
            path: s.path.clone(),
            reason: s.reason.clone(),
        })
        .collect()
}

/// Avanzamento con il tempo stimato, dalla velocità media finora.
fn progress_view(
    done: usize,
    total: usize,
    bytes_done: u64,
    bytes_total: u64,
    current: &str,
    started: Instant,
) -> ProgressView {
    let elapsed = started.elapsed().as_secs_f64();
    let (part, whole) = if bytes_total > 0 {
        (bytes_done as f64, bytes_total as f64)
    } else {
        (done as f64, total as f64)
    };
    let eta = (elapsed > 2.0 && part > 0.0).then(|| (elapsed * (whole - part) / part) as u64);
    ProgressView {
        done,
        total,
        bytes_done,
        bytes_total,
        current: current.to_string(),
        eta,
    }
}
