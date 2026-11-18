//! System tray: battery icon, tooltip and a context menu. Each driver contributes a section
//! through `Driver::tray` (header + quick controls); item ids are prefixed with the driver id.

use crate::{show_main, AppState};
use periph_core::{Tray, TrayItem};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;

const ICON: u32 = 32;

/// Windows taskbar theme (read once): true when the taskbar uses the light theme.
fn light_taskbar() -> bool {
    static LIGHT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *LIGHT.get_or_init(|| {
        std::process::Command::new("reg")
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "/v", "SystemUsesLightTheme"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("0x1"))
            .unwrap_or(false)
    })
}

/// Tray battery in the VT2 Icons style (24-grid SVG, rasterized with anti-aliasing):
/// 4 level cells, bolt while charging, grey with a dash when disconnected.
pub fn battery_icon(pct: Option<u8>, charging: bool) -> Image<'static> {
    let ink = if light_taskbar() { "#27272A" } else { "#F4F4F5" };
    let outline = if pct.is_none() { "#8A8A93" } else { ink };
    let inner = match (pct, charging) {
        (None, _) => r##"<path d="M8.5 12h6" stroke="#8A8A93" stroke-width="2.2" stroke-linecap="round"/>"##.to_string(),
        (Some(_), true) => r##"<path d="M12.9 7.4 8.6 12.7h3.5l-1.4 3.9 4.7-5.5h-3.5z" fill="#4ADE80"/>"##.to_string(),
        (Some(p), false) => {
            let color = if p > 50 { "#4ADE80" } else if p > 20 { "#FACC15" } else { "#F87171" };
            let cells = ((p.min(100) as u32).div_ceil(25)).max(1);
            (0..cells)
                .map(|i| format!(r#"<rect x="{:.2}" y="9" width="2.7" height="6" rx="0.7" fill="{color}"/>"#, 4.0 + i as f32 * 3.6))
                .collect()
        }
    };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke-linecap="round" stroke-linejoin="round">
<rect x="1.5" y="6.5" width="18.5" height="11" rx="3" stroke="{outline}" stroke-width="2.2"/>
<path d="M22.4 10.25v3.5" stroke="{outline}" stroke-width="2.2"/>
{inner}</svg>"#
    );

    use resvg::{tiny_skia, usvg};
    let render = || -> Option<Vec<u8>> {
        let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).ok()?;
        let mut pixmap = tiny_skia::Pixmap::new(ICON, ICON)?;
        let s = ICON as f32 / 24.0;
        resvg::render(&tree, tiny_skia::Transform::from_scale(s, s), &mut pixmap.as_mut());
        // tiny-skia stores premultiplied RGBA; the tray wants straight alpha
        Some(pixmap.pixels().iter().flat_map(|p| { let c = p.demultiply(); [c.red(), c.green(), c.blue(), c.alpha()] }).collect())
    };
    Image::new_owned(render().unwrap_or_else(|| vec![0; (ICON * ICON * 4) as usize]), ICON, ICON)
}

#[cfg(test)]
mod tests {
    /// Dumps raw RGBA previews to target/tray-preview/*.rgba (convert with tools/rgba_to_png.py).
    #[test]
    fn render_tray_variants() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tray-preview");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, pct, chg) in [("full", Some(90), false), ("mid", Some(45), false), ("low", Some(12), false), ("charging", Some(60), true), ("off", None, false)] {
            let img = super::battery_icon(pct, chg);
            assert_eq!(img.rgba().len(), (super::ICON * super::ICON * 4) as usize);
            assert!(img.rgba().chunks(4).any(|p| p[3] > 0), "{name} is empty");
            std::fs::write(dir.join(format!("{name}.rgba")), img.rgba()).unwrap();
        }
    }
}

/// Every driver's tray section, in registry order: `(driver id, section)`.
fn sections(app: &AppHandle) -> Vec<(&'static str, Tray)> {
    let state = app.state::<AppState>();
    state
        .slots
        .iter()
        .filter_map(|s| {
            let snap = s.snap.lock().unwrap().clone();
            s.driver.tray(&snap).map(|t| (s.driver.info().id, t))
        })
        .collect()
}

/// Update tray icon, tooltip and (if anything changed, or `force`) the context menu.
pub fn refresh(app: &AppHandle, force: bool) {
    let Some(icon) = app.tray_by_id("main") else { return };
    let secs = sections(app);
    let tip = if secs.is_empty() { "Periph".to_string() } else { secs.iter().map(|(_, t)| t.header.as_str()).collect::<Vec<_>>().join("
") };
    let _ = icon.set_tooltip(Some(tip));
    let battery = secs.iter().find_map(|(_, t)| t.battery);
    let _ = icon.set_icon(Some(battery_icon(battery.map(|b| b.0), battery.is_some_and(|b| b.1))));

    let auto = app.autolaunch().is_enabled().unwrap_or(false);
    let key = format!("{auto}|{:?}", secs.iter().map(|(id, t)| (id, &t.header, &t.items)).collect::<Vec<_>>());
    let state = app.state::<AppState>();
    let mut last = state.menu_key.lock().unwrap();
    if force || *last != key {
        if let Ok(menu) = build_menu(app) {
            let _ = icon.set_menu(Some(menu));
            *last = key;
        }
    }
}

fn item(app: &AppHandle, dev: &str, it: &TrayItem) -> tauri::Result<Box<dyn IsMenuItem<Wry>>> {
    Ok(match it {
        TrayItem::Action { id, label } => Box::new(MenuItem::with_id(app, format!("{dev}/{id}"), label, true, None::<&str>)?),
        TrayItem::Check { id, label, checked } => Box::new(CheckMenuItem::with_id(app, format!("{dev}/{id}"), label, true, *checked, None::<&str>)?),
        TrayItem::Separator => Box::new(PredefinedMenuItem::separator(app)?),
        TrayItem::Submenu { label, items } => {
            let sub = Submenu::new(app, label, true)?;
            for x in items {
                sub.append(item(app, dev, x)?.as_ref())?;
            }
            Box::new(sub)
        }
    })
}

pub fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let auto = app.autolaunch().is_enabled().unwrap_or(false);
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, "open", "Abrir Periph", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    for (dev, t) in sections(app) {
        menu.append(&MenuItem::with_id(app, format!("{dev}/header"), &t.header, false, None::<&str>)?)?;
        for it in &t.items {
            menu.append(item(app, dev, it)?.as_ref())?;
        }
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&CheckMenuItem::with_id(app, "autostart", "Iniciar com o Windows", true, auto, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?)?;
    Ok(menu)
}

pub fn on_menu(app: &AppHandle, e: MenuEvent) {
    match e.id.as_ref() {
        "open" => show_main(app),
        "quit" => app.exit(0),
        "autostart" => {
            let al = app.autolaunch();
            let _ = if al.is_enabled().unwrap_or(false) { al.disable() } else { al.enable() };
        }
        id => {
            if let Some((dev, item)) = id.split_once('/') {
                let state = app.state::<AppState>();
                if let Some(slot) = state.slot(dev) {
                    let snap = slot.snap.lock().unwrap().clone();
                    if let Some(cmd) = slot.driver.tray_command(item, &snap) {
                        let _ = slot.tx.lock().unwrap().send(cmd);
                    }
                }
            }
        }
    }
    // check items toggle themselves visually; resync with real state
    refresh(app, true);
}
