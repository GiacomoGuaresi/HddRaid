//! App HddRaid: finestra, icona nella barra e lavoro in background.

mod engine;
mod settings;
mod texts;
mod tray;
mod view;

use std::collections::BTreeMap;
use std::sync::Arc;

use hddraid_core::Side;
use tauri::{AppHandle, Manager, State, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use engine::{Cmd, Shared};
use settings::Settings;
use view::{AnswerView, HistoryEvent};

/// Argomento con cui l'avvio automatico lancia l'app: senza finestra.
const HIDDEN_ARG: &str = "--hidden";

type Engine<'a> = State<'a, Arc<Shared>>;

#[tauri::command]
fn core_version() -> &'static str {
    hddraid_core::version()
}

#[tauri::command]
fn get_state(engine: Engine) -> view::State {
    engine.state.lock().unwrap().clone()
}

#[tauri::command]
fn compare(engine: Engine) {
    engine.send(Cmd::Compare);
}

#[tauri::command]
fn sync(engine: Engine, answers: BTreeMap<String, AnswerView>) {
    engine.send(Cmd::Sync(answers));
}

#[tauri::command]
fn eject(engine: Engine) {
    engine.send(Cmd::Eject);
}

#[tauri::command]
fn start_check(engine: Engine) {
    engine.send(Cmd::StartCheck);
}

#[tauri::command]
fn stop_check(engine: Engine) {
    engine
        .cancel
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

#[tauri::command]
fn postpone_check(engine: Engine) {
    engine.send(Cmd::PostponeCheck);
}

#[tauri::command]
fn repair(engine: Engine) {
    engine.send(Cmd::Repair);
}

#[tauri::command]
fn dismiss(engine: Engine) {
    engine.send(Cmd::Dismiss);
}

#[tauri::command]
fn rename_disk(engine: Engine, side: Side, label: String) -> Result<(), String> {
    hddraid_core::volumes::check_label(&label).map_err(|e| format!("{e:?}"))?;
    engine.send(Cmd::Rename(side, label));
    Ok(())
}

/// Apre un file di un disco (o lo mostra nella sua cartella), per capire
/// di quale file parla una domanda.
#[tauri::command]
fn open_file(
    app: AppHandle,
    engine: Engine,
    side: Side,
    path: String,
    reveal: bool,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    // Il percorso arriva dall'interfaccia: deve restare dentro il disco.
    if path.split('/').any(|part| part == ".." || part.is_empty()) {
        return Err("percorso non valido".into());
    }
    let root = engine
        .state
        .lock()
        .unwrap()
        .disks
        .get(&side)
        .cloned()
        .flatten()
        .map(|d| d.root)
        .ok_or("disco non collegato")?;
    let full = hddraid_core::paths::native(std::path::Path::new(&root), &path);
    let result = if reveal {
        app.opener().reveal_item_in_dir(&full)
    } else {
        app.opener().open_path(full.to_string_lossy(), None::<&str>)
    };
    result.map_err(|e| e.to_string())
}

/// Formatta un volume in exFAT e lo mette nella coppia al posto del disco
/// `side`. Cancella tutto il volume: l'interfaccia chiede conferma prima.
#[tauri::command]
fn format_disk(engine: Engine, root: String, side: Side, label: String) -> Result<(), String> {
    hddraid_core::volumes::check_label(&label).map_err(|e| format!("{e:?}"))?;
    engine.send(Cmd::Format {
        root: root.into(),
        side,
        label,
    });
    Ok(())
}

#[tauri::command]
fn history(engine: Engine) -> Vec<HistoryEvent> {
    engine.history(500)
}

#[tauri::command]
fn get_settings(engine: Engine) -> Settings {
    engine.config.lock().unwrap().settings.clone()
}

#[tauri::command]
fn set_settings(app: AppHandle, engine: Engine, settings: Settings) -> Result<(), String> {
    let autostart = settings.autostart;
    {
        let mut config = engine.config.lock().unwrap();
        config.settings = settings;
        config.save(&engine.config_dir).map_err(|e| e.to_string())?;
    }
    set_autostart(&app, autostart);
    engine.send(Cmd::Settings);
    Ok(())
}

fn set_autostart(app: &AppHandle, enabled: bool) {
    // In sviluppo si registrerebbe il binario di debug.
    if cfg!(debug_assertions) {
        return;
    }
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    if let Err(e) = result {
        log::warn!("avvio automatico non impostato: {e}");
    }
}

/// Mostra e porta in primo piano la finestra.
pub fn show_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_window(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(2_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let config_dir = app.path().app_config_dir()?;
            let work_dir = app.path().app_data_dir()?.join("work");
            let engine = engine::start(handle.clone(), config_dir, work_dir);
            tray::create(&handle, &engine.texts())?;

            let first_run = {
                let mut config = engine.config.lock().unwrap();
                let first = !config.autostart_configured;
                config.autostart_configured = true;
                let _ = config.save(&engine.config_dir);
                first.then_some(config.settings.autostart)
            };
            if let Some(enabled) = first_run {
                set_autostart(&handle, enabled);
            }
            app.manage(engine);

            if std::env::args().any(|a| a == HIDDEN_ARG) {
                #[cfg(target_os = "macos")]
                let _ = handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
            } else {
                show_window(&handle);
            }
            log::info!("HddRaid {} avviato", hddraid_core::version());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Chiudere la finestra la nasconde: l'app resta nella barra.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                #[cfg(target_os = "macos")]
                let _ = window
                    .app_handle()
                    .set_activation_policy(tauri::ActivationPolicy::Accessory);
            }
        })
        .invoke_handler(tauri::generate_handler![
            core_version,
            get_state,
            compare,
            sync,
            eject,
            start_check,
            stop_check,
            postpone_check,
            repair,
            dismiss,
            rename_disk,
            open_file,
            format_disk,
            history,
            get_settings,
            set_settings,
        ])
        .run(tauri::generate_context!())
        .expect("errore all'avvio di HddRaid");
}
