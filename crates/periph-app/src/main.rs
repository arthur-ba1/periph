#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Periph — Tauri shell that runs every registered device driver (see `drivers/`) on its own
//! worker thread and bridges it to the UI. `periph --tray` starts hidden.
//!
//! UI contract (all devices):
//! - `devices()` -> `[{ id, name, kind, snapshot }]`
//! - `device_send(device, cmd)` queues a driver-specific JSON command
//! - event `"device"`: `{ device, type: "connected" | "disconnected" | "config" | "status" | "error" | "done", ... }`

mod drivers;
mod tray;

use periph_core::{serde_json::Value, DeviceInfo, Driver, Event, Snapshot};
use serde::Serialize;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

/// One registered driver plus its live state.
pub struct Slot {
    pub driver: Arc<dyn Driver>,
    pub tx: Mutex<Sender<Value>>,
    pub snap: Mutex<Snapshot>,
}

pub struct AppState {
    pub slots: Vec<Slot>,
    pub menu_key: Mutex<String>,
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

#[tauri::command]
fn device_send(state: State<AppState>, device: String, cmd: Value) -> Result<(), String> {
    let slot = state.slot(&device).ok_or_else(|| format!("dispositivo desconhecido: {device}"))?;
    slot.tx.lock().unwrap().send(cmd).map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn toggle_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            show_main(app);
        }
    }
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
    let start_hidden = std::env::args().any(|a| a == "--tray");
    let mut receivers = Vec::new();
    let slots = drivers::all()
        .into_iter()
        .map(|d| {
            let (tx, rx) = channel::<Value>();
            receivers.push(rx);
            Slot { driver: Arc::from(d), tx: Mutex::new(tx), snap: Mutex::new(Snapshot::default()) }
        })
        .collect();

    tauri::Builder::default()
        // Must be first: a second launch just focuses the running instance (only one may own the HID channels).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--tray"])))
        .manage(AppState { slots, menu_key: Mutex::new(String::new()) })
        .invoke_handler(tauri::generate_handler![devices, device_send, hide_window])
        .setup(move |app| {
            let handle = app.handle().clone();
            migrate_autostart(&handle);
            TrayIconBuilder::with_id("main")
                .icon(tray::battery_icon(None, false))
                .tooltip("Periph")
                .menu(&tray::build_menu(&handle)?)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| tray::on_menu(app, e))
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        toggle_main(tray.app_handle());
                    }
                })
                .build(app)?;

            // One worker thread per driver.
            for (i, rx) in receivers.into_iter().enumerate() {
                let h = handle.clone();
                let driver = h.state::<AppState>().slots[i].driver.clone();
                let id = driver.info().id;
                std::thread::spawn(move || {
                    periph_core::run(driver.as_ref(), rx, |e| {
                        h.state::<AppState>().slots[i].snap.lock().unwrap().update(&e);
                        if !matches!(e, Event::Done | Event::Error { .. }) {
                            tray::refresh(&h, false);
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
        .on_window_event(|window, event| {
            // Closing the window keeps the app in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Periph");
}
