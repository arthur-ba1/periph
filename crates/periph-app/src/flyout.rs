//! The tray flyout: one hidden, frameless webview window (label "tray") anchored to the tray.
//! Left click on an icon shows that device's quick settings, right click the app menu and a
//! double click opens the main window on the device. The frontend (`app/src/tray/`) renders
//! the payload of [`state`], measures itself and calls [`flyout_layout`] to be placed and shown.

use crate::{open_device, tray, AppState};
use periph_core::{Kind, TrayItem};
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
use tauri_plugin_autostart::ManagerExt;

pub const LABEL: &str = "tray";

/// A click on the icon that hid the flyout (by taking focus) closes it instead of reopening it.
const TOGGLE_WINDOW: Duration = Duration::from_millis(300);
/// The second button-up of a double click is not a new click.
const DOUBLE_WINDOW: Duration = Duration::from_millis(500);
/// Gap between the flyout and the taskbar or icon, in logical pixels.
const GAP: f64 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Quick,
    Menu,
}

/// Physical screen rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Area {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Clone, Copy, Debug)]
enum Anchor {
    /// The tray icon's rectangle (quick settings).
    Icon(Area),
    /// The cursor (menu).
    Point(f64, f64),
}

struct Request {
    seq: u64,
    mode: Mode,
    icon: String,
    anchor: Anchor,
}

#[derive(Default)]
pub struct FlyoutState(Mutex<Inner>);

#[derive(Default)]
struct Inner {
    seq: u64,
    request: Option<Request>,
    visible: bool,
    hidden: Option<(Instant, Mode, String)>,
    double: Option<Instant>,
    sent: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    seq: u64,
    mode: Mode,
    /// Device of the clicked icon (`None` for the plain Periph icon).
    device: Option<&'static str>,
    devices: Vec<PanelDevice>,
    autostart: bool,
    locale: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PanelDevice {
    id: &'static str,
    kind: Kind,
    name: String,
    connected: bool,
    header: String,
    battery: Option<Battery>,
    items: Vec<TrayItem>,
}

#[derive(Clone, Serialize)]
struct Battery {
    percent: u8,
    charging: bool,
}

fn device_of(icon: &str) -> Option<&str> {
    icon.strip_prefix("device/")
}

fn panel(app: &AppHandle, request: &Request) -> Panel {
    let devices: Vec<_> = tray::devices(app)
        .into_iter()
        .map(|d| PanelDevice {
            id: d.info.id,
            kind: d.info.kind,
            connected: d.connected,
            header: d.header().to_owned(),
            battery: d.section.as_ref().and_then(|s| s.battery).map(|(percent, charging)| Battery { percent, charging }),
            items: d.items().to_vec(),
            name: d.name,
        })
        .collect();
    let device = device_of(&request.icon).and_then(|id| devices.iter().find(|d| d.id == id)).map(|d| d.id);
    Panel {
        seq: request.seq,
        mode: request.mode,
        device,
        devices,
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        locale: periph_core::i18n::locale(),
    }
}

fn flyout(app: &AppHandle) -> std::sync::MutexGuard<'_, Inner> {
    app.state::<AppState>().inner().flyout.0.lock().unwrap()
}

/// Mouse events of every tray icon.
pub fn on_tray_event(app: &AppHandle, event: TrayIconEvent) {
    let now = Instant::now();
    match event {
        TrayIconEvent::Click { id, rect, position, button, button_state: MouseButtonState::Up } => {
            let icon = id.0;
            let mode = match button {
                MouseButton::Left => Mode::Quick,
                MouseButton::Right => Mode::Menu,
                MouseButton::Middle => return,
            };
            let mut inner = flyout(app);
            if mode == Mode::Quick && inner.double.is_some_and(|at| now - at < DOUBLE_WINDOW) {
                return;
            }
            let reopening = inner.hidden.as_ref().is_some_and(|(at, m, i)| now - *at < TOGGLE_WINDOW && *m == mode && *i == icon);
            let same = inner.request.as_ref().is_some_and(|r| r.mode == mode && r.icon == icon);
            if reopening || (inner.visible && same) {
                inner.hidden = None;
                drop(inner);
                hide(app);
                return;
            }
            // Tray rects are reported in physical pixels (the scale only matters for logical ones).
            let pos = rect.position.to_physical::<f64>(1.0);
            let size = rect.size.to_physical::<f64>(1.0);
            let anchor = match mode {
                Mode::Quick => Anchor::Icon(Area { x: pos.x, y: pos.y, w: size.width, h: size.height }),
                Mode::Menu => Anchor::Point(position.x, position.y),
            };
            inner.seq += 1;
            inner.request = Some(Request { seq: inner.seq, mode, icon, anchor });
            inner.sent.clear();
            drop(inner);
            notify(app);
        }
        TrayIconEvent::DoubleClick { id, button: MouseButton::Left, .. } => {
            flyout(app).double = Some(now);
            hide(app);
            open_device(app, device_of(&id.0));
        }
        _ => {}
    }
}

/// Send the current payload to the flyout if it changed (called on every tray refresh).
pub fn notify(app: &AppHandle) {
    let mut inner = flyout(app);
    let Some(request) = inner.request.as_ref() else { return };
    let payload = panel(app, request);
    let json = serde_json::to_string(&payload).unwrap_or_default();
    if json == inner.sent {
        return;
    }
    inner.sent = json;
    drop(inner);
    let _ = app.emit_to(LABEL, "tray-panel", payload);
}

pub fn hide(app: &AppHandle) {
    let mut inner = flyout(app);
    inner.request = None;
    if std::mem::take(&mut inner.visible) {
        drop(inner);
        if let Some(window) = app.get_webview_window(LABEL) {
            let _ = window.hide();
        }
        // Drop the old content so it cannot flash when the window shows the next panel.
        let _ = app.emit_to(LABEL, "tray-panel-hidden", ());
    }
}

/// The flyout lost focus (a click elsewhere, including on its own tray icon).
pub fn on_blur(app: &AppHandle) {
    let mut inner = flyout(app);
    let shown = inner.request.as_ref().filter(|_| inner.visible).map(|r| (r.mode, r.icon.clone()));
    if let Some((mode, icon)) = shown {
        inner.hidden = Some((Instant::now(), mode, icon));
    }
    drop(inner);
    hide(app);
}

/// Where to put a `w`×`h` flyout (physical pixels) inside the monitor's work area.
fn place(anchor: Anchor, w: f64, h: f64, work: Area, gap: f64) -> (f64, f64) {
    let (right, bottom) = (work.x + work.w, work.y + work.h);
    let (x, y) = match anchor {
        Anchor::Point(px, py) => {
            let x = if px + w > right { px - w } else { px };
            let y = if py - h < work.y { py } else { py - h };
            (x, y)
        }
        Anchor::Icon(icon) => {
            let (cx, cy) = (icon.x + icon.w / 2.0, icon.y + icon.h / 2.0);
            if cy >= bottom {
                (cx - w / 2.0, bottom - h - gap) // taskbar at the bottom
            } else if cy <= work.y {
                (cx - w / 2.0, work.y + gap) // top
            } else if cx <= work.x {
                (work.x + gap, cy - h / 2.0) // left
            } else if cx >= right {
                (right - w - gap, cy - h / 2.0) // right
            } else if icon.y - h - gap >= work.y {
                (cx - w / 2.0, icon.y - h - gap) // overflow area: above the icon
            } else {
                (cx - w / 2.0, icon.y + icon.h + gap)
            }
        }
    };
    let clamp = |v: f64, lo: f64, hi: f64| if hi < lo { lo } else { v.clamp(lo, hi) };
    (clamp(x, work.x + gap, right - w - gap).round(), clamp(y, work.y + gap, bottom - h - gap).round())
}

#[tauri::command]
pub fn flyout_state(app: AppHandle) -> Option<Panel> {
    let inner = flyout(&app);
    inner.request.as_ref().map(|request| panel(&app, request))
}

/// The frontend rendered request `seq` at `width`×`height` CSS pixels: place and show it.
#[tauri::command]
pub fn flyout_layout(app: AppHandle, seq: u64, width: f64, height: f64) -> Result<(), String> {
    let mut inner = flyout(&app);
    let Some(request) = inner.request.as_ref().filter(|r| r.seq == seq) else { return Ok(()) };
    let window = app.get_webview_window(LABEL).ok_or("no tray window")?;
    let (ax, ay) = match request.anchor {
        Anchor::Icon(a) => (a.x + a.w / 2.0, a.y + a.h / 2.0),
        Anchor::Point(x, y) => (x, y),
    };
    let monitor = app
        .monitor_from_point(ax, ay)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or("no monitor")?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let work = Area { x: area.position.x.into(), y: area.position.y.into(), w: area.size.width.into(), h: area.size.height.into() };
    let (w, h) = ((width * scale).ceil(), (height * scale).ceil());
    let (x, y) = place(request.anchor, w, h, work, GAP * scale);
    let e = |e: tauri::Error| e.to_string();
    window.set_position(PhysicalPosition::new(x as i32, y as i32)).map_err(e)?;
    window.set_size(PhysicalSize::new(w as u32, h as u32)).map_err(e)?;
    if !inner.visible {
        inner.visible = true;
        drop(inner);
        window.show().map_err(e)?;
        window.set_focus().map_err(e)?;
    }
    Ok(())
}

#[tauri::command]
pub fn flyout_hide(app: AppHandle) {
    hide(&app);
}

/// Open the main window, on `device` if given.
#[tauri::command]
pub fn flyout_open(app: AppHandle, device: Option<String>) {
    hide(&app);
    open_device(&app, device.as_deref());
}

/// A quick setting of `device` was used (an item id from its [`periph_core::Tray`]).
#[tauri::command]
pub fn flyout_command(app: AppHandle, device: String, item: String) {
    let state = app.state::<AppState>();
    if let Some(slot) = state.slot(&device) {
        let snap = slot.snap.lock().unwrap().clone();
        if let Some(cmd) = slot.driver.tray_command(&item, &snap) {
            let _ = slot.tx.lock().unwrap().send(cmd);
        }
    }
}

#[tauri::command]
pub fn flyout_autostart(app: AppHandle, enabled: bool) {
    let autolaunch = app.autolaunch();
    let _ = if enabled { autolaunch.enable() } else { autolaunch.disable() };
    notify(&app);
}

#[tauri::command]
pub fn flyout_quit(app: AppHandle) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Area = Area { x: 0.0, y: 0.0, w: 1920.0, h: 1032.0 };

    #[test]
    fn quick_sits_above_a_bottom_taskbar_centered_on_the_icon() {
        let icon = Anchor::Icon(Area { x: 1700.0, y: 1040.0, w: 24.0, h: 40.0 });
        assert_eq!(place(icon, 320.0, 300.0, WORK, 10.0), (1552.0, 722.0));
        // near the screen edge it stays inside the work area
        let edge = Anchor::Icon(Area { x: 1890.0, y: 1040.0, w: 24.0, h: 40.0 });
        assert_eq!(place(edge, 320.0, 300.0, WORK, 10.0).0, 1590.0);
    }

    #[test]
    fn quick_follows_side_and_top_taskbars() {
        let left = Area { x: 60.0, ..WORK };
        assert_eq!(place(Anchor::Icon(Area { x: 10.0, y: 900.0, w: 40.0, h: 24.0 }), 320.0, 300.0, left, 10.0), (70.0, 722.0));
        let top = Area { y: 48.0, h: 1032.0, ..WORK };
        assert_eq!(place(Anchor::Icon(Area { x: 1000.0, y: 8.0, w: 24.0, h: 32.0 }), 320.0, 300.0, top, 10.0), (852.0, 58.0));
    }

    #[test]
    fn menu_opens_up_from_the_cursor_and_flips_at_edges() {
        assert_eq!(place(Anchor::Point(1000.0, 1050.0), 240.0, 200.0, WORK, 10.0), (1000.0, 822.0));
        assert_eq!(place(Anchor::Point(1900.0, 1050.0), 240.0, 200.0, WORK, 10.0), (1660.0, 822.0));
        assert_eq!(place(Anchor::Point(500.0, 100.0), 240.0, 200.0, WORK, 10.0), (500.0, 100.0));
    }
}
