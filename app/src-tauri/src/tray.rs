//! Icona nella barra di sistema / dei menu: stato a colpo d'occhio e menu
//! con Apri, Espelli, Esci.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::engine::Cmd;
use crate::engine::Shared;
use crate::texts::Texts;
use crate::view::{Busy, State};

const ID: &str = "main";

/// Colore del pallino di stato.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dot {
    Grey,
    Yellow,
    Teal,
    Green,
    Red,
}

impl Dot {
    fn rgb(self) -> [u8; 3] {
        match self {
            Dot::Grey => [0x9d, 0x94, 0x8a],
            Dot::Yellow => [0xf2, 0xa9, 0x1e],
            Dot::Teal => [0x22, 0xb5, 0xaa],
            Dot::Green => [0x3c, 0xb3, 0x68],
            Dot::Red => [0xe5, 0x48, 0x4d],
        }
    }
}

/// Colore e testo dell'icona per lo stato attuale.
fn status(state: &State, texts: &Texts) -> (Dot, String) {
    let label = |side| {
        state
            .pair
            .as_ref()
            .and_then(|p| p.labels.get(&side).cloned())
            .unwrap_or_default()
    };
    match &state.busy {
        Some(Busy::Syncing { progress }) => return (Dot::Teal, texts.copying(progress.percent())),
        Some(Busy::Checking { progress }) | Some(Busy::Repairing { progress }) => {
            return (Dot::Teal, texts.checking(progress.percent()));
        }
        Some(Busy::Comparing) => return (Dot::Teal, texts.comparing()),
        Some(Busy::Ejecting) | None => {}
    }
    let present: Vec<_> = state
        .disks
        .iter()
        .filter_map(|(side, d)| d.as_ref().map(|_| *side))
        .collect();
    match present.len() {
        0 => (Dot::Grey, texts.no_disks()),
        1 => (Dot::Yellow, texts.connect_other(&label(present[0].other()))),
        _ => match &state.summary {
            Some(s) if !s.questions.is_empty() => (Dot::Red, texts.questions(s.questions.len())),
            Some(s) => (Dot::Teal, texts.to_align(s.changes())),
            None => (Dot::Green, texts.in_sync()),
        },
    }
}

pub fn create(app: &AppHandle, texts: &Texts) -> tauri::Result<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", texts.open(), true, None::<&str>)?,
            &MenuItem::with_id(app, "eject", texts.eject(), true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", texts.quit(), true, None::<&str>)?,
        ],
    )?;
    TrayIconBuilder::with_id(ID)
        .icon(icon(Dot::Grey))
        .tooltip(texts.no_disks())
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => crate::show_window(app),
            "eject" => app.state::<std::sync::Arc<Shared>>().send(Cmd::Eject),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::show_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Aggiorna icona e testo. Si chiama a ogni cambio di stato.
pub fn update(app: &AppHandle, state: &State, texts: &Texts) {
    let Some(tray) = app.tray_by_id(ID) else {
        return;
    };
    let (dot, text) = status(state, texts);
    let _ = tray.set_icon(Some(icon(dot)));
    let _ = tray.set_tooltip(Some(&text));
}

const SIZE: usize = 64;

/// Un disco stilizzato con il pallino di stato in basso a destra.
fn icon(dot: Dot) -> Image<'static> {
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let grey = [0x8a, 0x84, 0x7d];
    let color = dot.rgb();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // Contorno del disco: rettangolo arrotondato, spessore 5.
            let d = rounded_rect(px, py, 32.0, 32.0, 27.0, 19.0, 9.0);
            let mut alpha = coverage(d.abs() - 2.5);
            // Fessura sul fronte.
            let slot = rounded_rect(px, py, 22.0, 39.0, 8.0, 2.0, 2.0);
            alpha = alpha.max(coverage(slot));
            // Pallino, con un bordo vuoto che lo stacca dal disco.
            let dist = ((px - 47.0).powi(2) + (py - 44.0).powi(2)).sqrt();
            alpha *= 1.0 - coverage(dist - 16.0);
            let dot_alpha = coverage(dist - 12.0);
            let i = (y * SIZE + x) * 4;
            let (rgb, a) = if dot_alpha > 0.0 {
                (color, dot_alpha.max(alpha))
            } else {
                (grey, alpha)
            };
            rgba[i..i + 3].copy_from_slice(&rgb);
            rgba[i + 3] = (a * 255.0).round() as u8;
        }
    }
    Image::new_owned(rgba, SIZE as u32, SIZE as u32)
}

/// Distanza con segno da un rettangolo arrotondato (centro, semiassi, raggio).
fn rounded_rect(x: f32, y: f32, cx: f32, cy: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = (x - cx).abs() - (hw - r);
    let qy = (y - cy).abs() - (hh - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}

/// Copertura di un pixel a distanza `d` dal bordo (negativa = dentro).
fn coverage(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}
