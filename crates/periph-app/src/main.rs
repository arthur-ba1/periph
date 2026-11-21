#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Periph — Tauri shell that runs every registered device driver (see `drivers/`) on its own
//! worker thread and bridges it to the UI. `periph --tray` starts hidden.
//!
//! UI contract (all devices):
//! - `devices()` -> `[{ id, name, kind, snapshot }]`
//! - `device_send(device, cmd)` queues a driver-specific JSON command
//! - `device_packages()` -> `{ packages: [{ manifest, device, locales }], failed: [{ dir, error }] }`
//! - `set_locale(locale)` switches backend text (tray, errors) to the UI language
//! - event `"device"`: `{ device, type: "connected" | "disconnected" | "config" | "status" | "error" | "done", ... }`
//! - event `"open-device"` (main window): `"<id>"`, show that device (tray double click)
//! - event `"window-visible"` (main window): `bool`, hidden to the tray / minimized or back;
//!   while hidden, live `status` events are thinned out (the snapshot stays current)
//! - `flyout_*` commands and event `"tray-panel"` drive the tray window, see `flyout.rs`

mod drivers;
mod flyout;
mod packages;
mod tray;

use periph_core::{serde_json::Value, DeviceInfo, Driver, Event, Snapshot};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

/// One registered driver plus its live state.
pub struct Slot {
    pub driver: Arc<dyn Driver>,
    pub tx: Mutex<Sender<Value>>,
    pub snap: Mutex<Snapshot>,
    /// Last tray refresh caused by a status event, and whether a trailing one is scheduled.
    tray_at: Mutex<Option<Instant>>,
    tray_pending: AtomicBool,
    /// Last status event sent to the UI while the main window was hidden.
    ui_at: Mutex<Option<Instant>>,
}

/// Live status (DualSense input, battery) can arrive many times a second. The tray only shows
/// battery and tooltips, so it follows status at most this often (the last value always lands).
const TRAY_STATUS_EVERY: Duration = Duration::from_secs(1);
/// Status events sent to a hidden main window (it gets the full snapshot when shown).
const HIDDEN_STATUS_EVERY: Duration = Duration::from_secs(2);

/// True (and the stamp moved to now) when `every` has passed since the stamp.
fn due(stamp: &Mutex<Option<Instant>>, every: Duration) -> bool {
    let mut at = stamp.lock().unwrap();
    let now = Instant::now();
    if at.is_some_and(|t| now.duration_since(t) < every) {
        return false;
    }
    *at = Some(now);
    true
}

impl Slot {
    /// Tray refresh for a status event: right away if the last one is old enough, otherwise
    /// once the interval has passed (one trailing refresh per interval).
    fn refresh_tray_for_status(&self, app: &AppHandle) {
        if due(&self.tray_at, TRAY_STATUS_EVERY) {
            tray::refresh(app, false);
        } else if !self.tray_pending.swap(true, Ordering::AcqRel) {
            let h = app.clone();
            let id = self.driver.info().id;
            std::thread::spawn(move || {
                std::thread::sleep(TRAY_STATUS_EVERY);
                if let Some(slot) = h.state::<AppState>().slot(id) {
                    slot.tray_pending.store(false, Ordering::Release);
                    *slot.tray_at.lock().unwrap() = Some(Instant::now());
                }
                tray::refresh(&h, false);
            });
        }
    }
}

pub struct AppState {
    pub slots: Vec<Slot>,
    pub tray: tray::TrayState,
    pub flyout: flyout::FlyoutState,
    /// The main window is on screen (not hidden to the tray, not minimized).
    pub main_visible: AtomicBool,
}

impl AppState {
    pub fn slot(&self, id: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.driver.info().id == id)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceEntry {
    #[serde(flatten)]
    info: DeviceInfo,
    snapshot: Snapshot,
}

#[derive(Serialize, Clone)]
struct DeviceEvent<'a> {
    device: &'static str,
    #[serde(flatten)]
    event: &'a Event,
}

#[tauri::command]
fn devices(state: State<AppState>) -> Vec<DeviceEntry> {
    state.slots.iter().map(|s| DeviceEntry { info: s.driver.info(), snapshot: s.snap.lock().unwrap().clone() }).collect()
}

/// UI payload of the installed device packages.
struct Installed(Value);

#[tauri::command]
fn device_packages(installed: State<Installed>) -> Value {
    installed.0.clone()
}

#[tauri::command]
fn device_send(state: State<AppState>, device: String, cmd: Value) -> Result<(), String> {
    let slot = state.slot(&device).ok_or_else(|| format!("dispositivo desconhecido: {device}"))?;
    slot.tx.lock().unwrap().send(cmd).map_err(|e| e.to_string())
}

/// The UI's language (e.g. "pt-BR"); used for the tray and backend messages.
#[tauri::command]
fn set_locale(app: AppHandle, locale: String) {
    periph_core::i18n::set_locale(&locale);
    tray::refresh(&app, true);
}

/// Translation catalogs embedded by build.rs: (lang, namespace, json).
const LOCALES: &[(&str, &str, &str)] = include!(concat!(env!("OUT_DIR"), "/locales.rs"));

#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    set_main_visible(&app, false);
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    set_main_visible(app, true);
}

/// Tell the main window whether it can be seen (its animation loops stop while it can't).
/// Coming back, it gets every device's current status, which was thinned out while hidden.
fn set_main_visible(app: &AppHandle, visible: bool) {
    let state = app.state::<AppState>();
    if state.main_visible.swap(visible, Ordering::AcqRel) == visible {
        return;
    }
    let _ = app.emit_to("main", "window-visible", visible);
    if visible {
        for slot in &state.slots {
            let status = slot.snap.lock().unwrap().status.clone();
            if let Some(status) = status {
                let event = Event::Status { status };
                let _ = app.emit_to("main", "device", DeviceEvent { device: slot.driver.info().id, event: &event });
            }
        }
    }
}

/// Show the main window, switched to `device` if given.
pub fn open_device(app: &AppHandle, device: Option<&str>) {
    if let Some(id) = device {
        let _ = app.emit_to("main", "open-device", id);
    }
    show_main(app);
}

/// The app used to be called "VT2 Control" (vt2-control.exe). Its "start with Windows" entry
/// would keep launching the old exe, so drop it and, if it was on, re-enable under the new name.
fn migrate_autostart(app: &tauri::AppHandle) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use tauri_plugin_autostart::ManagerExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const RUN: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
        let reg = |args: &[&str]| std::process::Command::new("reg").args(args).creation_flags(CREATE_NO_WINDOW).status();
        let had_old = reg(&["query", RUN, "/v", "VT2 Control"]).map(|s| s.success()).unwrap_or(false);
        if had_old && reg(&["delete", RUN, "/v", "VT2 Control", "/f"]).map(|s| s.success()).unwrap_or(false) {
            let _ = app.autolaunch().enable();
        }
    }
    #[cfg(not(windows))]
    let _ = app;
}

fn main() {
    for (lang, ns, json) in LOCALES {
        if let Err(e) = periph_core::i18n::load(lang, ns, json) {
            eprintln!("locale {ns}/{lang}: {e}");
        }
    }
    let start_hidden = std::env::args().any(|a| a == "--tray");
    let mut receivers = Vec::new();
    let mut all = drivers::all();
    let installed = Installed(packages::install(&mut all));
    let slots = all
        .into_iter()
        .map(|d| {
            let (tx, rx) = channel::<Value>();
            receivers.push(rx);
            Slot {
                driver: Arc::from(d),
                tx: Mutex::new(tx),
                snap: Mutex::new(Snapshot::default()),
                tray_at: Mutex::new(None),
                tray_pending: AtomicBool::new(false),
                ui_at: Mutex::new(None),
            }
        })
        .collect();

    tauri::Builder::default()
        // Must be first: a second launch just focuses the running instance (only one may own the HID channels).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--tray"])))
        .manage(installed)
        .manage(AppState {
            slots,
            tray: tray::TrayState::default(),
            flyout: flyout::FlyoutState::default(),
            main_visible: AtomicBool::new(!start_hidden),
        })
        .invoke_handler(tauri::generate_handler![
            devices,
            device_packages,
            device_send,
            set_locale,
            hide_window,
            flyout::flyout_state,
            flyout::flyout_layout,
            flyout::flyout_hide,
            flyout::flyout_open,
            flyout::flyout_command,
            flyout::flyout_autostart,
            flyout::flyout_quit,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            migrate_autostart(&handle);
            tray::install(&handle)?;

            // One worker thread per driver.
            for (i, rx) in receivers.into_iter().enumerate() {
                let h = handle.clone();
                let driver = h.state::<AppState>().slots[i].driver.clone();
                let id = driver.info().id;
                std::thread::spawn(move || {
                    periph_core::run(driver.as_ref(), rx, |e| {
                        let state = h.state::<AppState>();
                        let slot = &state.slots[i];
                        slot.snap.lock().unwrap().update(&e);
                        match e {
                            Event::Status { .. } => {
                                slot.refresh_tray_for_status(&h);
                                if !state.main_visible.load(Ordering::Acquire) && !due(&slot.ui_at, HIDDEN_STATUS_EVERY) {
                                    return;
                                }
                            }
                            Event::Done | Event::Error { .. } => {}
                            _ => tray::refresh(&h, false),
                        }
                        let _ = h.emit("device", DeviceEvent { device: id, event: &e });
                    });
                });
            }

            if !start_hidden {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing the window keeps the app in the tray.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
                if window.label() == "main" {
                    set_main_visible(window.app_handle(), false);
                }
            }
            // Minimize / restore (the window is resized to nothing and back).
            WindowEvent::Resized(_) if window.label() == "main" => {
                let shown = window.is_visible().unwrap_or(true) && !window.is_minimized().unwrap_or(false);
                set_main_visible(window.app_handle(), shown);
            }
            // The flyout behaves like a popup: any click elsewhere dismisses it.
            WindowEvent::Focused(false) if window.label() == flyout::LABEL => flyout::on_blur(window.app_handle()),
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running Periph");
}
