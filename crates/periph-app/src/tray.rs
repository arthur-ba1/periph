//! One system tray icon per connected device (battery first, device glyph as a hint) plus
//! tooltips. Clicks go to the flyout window (`flyout.rs`), which replaces the native menu.

use crate::{flyout, AppState};
use periph_core::{DeviceInfo, Kind, Tray, TrayItem};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::image::Image;
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

const ICON: u32 = 32;

#[derive(Default)]
pub struct TrayState {
    queued: AtomicBool,
    contents: Mutex<TrayContents>,
}

#[derive(Default)]
struct TrayContents {
    icons: BTreeMap<String, (IconSpec, bool)>,
}

/// Windows taskbar theme (read once).
fn light_taskbar() -> bool {
    static LIGHT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *LIGHT.get_or_init(|| {
        let mut command = std::process::Command::new("reg");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        command
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "/v", "SystemUsesLightTheme"])
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("0x1"))
    })
}

fn device_glyph(kind: Option<Kind>) -> &'static str {
    match kind {
        None => r#"<rect x="2.5" y="2.5" width="19" height="19" rx="5"/><path d="M9 17v-7m0 0a4 4 0 1 1 8 0 4 4 0 0 1-8 0Z"/>"#,
        Some(Kind::Mouse) => r#"<rect x="5.5" y="2.5" width="13" height="19" rx="6.5"/><path d="M12 3v4.5M5.7 10h12.6"/>"#,
        Some(Kind::Keyboard) => r#"<rect x="2" y="5.5" width="20" height="13" rx="3"/><path d="M6.5 10h.2m5.2 0h.2m5.2 0h.2M6.5 14h.2m4.1 0h6.7"/>"#,
        Some(Kind::Controller) => r#"<path d="M8 6h8c2.4 0 3.8 1.4 4.3 3.8l1 4.9c.5 2.6-1.8 4-3.5 2.5l-2.4-2.1H8.6l-2.4 2.1c-1.7 1.5-4 .1-3.5-2.5l1-4.9C4.2 7.4 5.6 6 8 6Z"/><path d="M8 8.7v4M6 10.7h4m5.3-1.3h.2m2.1 2.3h.2"/>"#,
        Some(Kind::Headset) => r#"<path d="M4 14v-2a8 8 0 0 1 16 0v2"/><rect x="3" y="12.5" width="4.5" height="6.5" rx="2"/><rect x="16.5" y="12.5" width="4.5" height="6.5" rx="2"/>"#,
        Some(Kind::Other) => r#"<rect x="3" y="5" width="18" height="14" rx="3"/><path d="M8 12h.1m3.9 0h.1m3.9 0h.1"/>"#,
    }
}

/// Tray icon: with a battery, the gauge fills most of the icon and the device glyph shrinks to
/// a hint above it, so the charge reads at 16 px; without one, the glyph alone.
fn device_svg(kind: Option<Kind>, battery: Option<(u8, bool)>, light: bool) -> String {
    let (ink, knock, green, amber, red) = if light {
        ("#242630", "#FFFFFF", "#14794B", "#946200", "#BD2939")
    } else {
        ("#F3F5FA", "#101114", "#53D597", "#FFD364", "#FF7885")
    };
    let glyph = device_glyph(kind);
    let Some((pct, charging)) = battery else {
        return format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><g fill="none" stroke="{ink}" stroke-width="2.15" stroke-linecap="round" stroke-linejoin="round">{glyph}</g></svg>"#
        );
    };
    let pct = pct.min(100);
    let low = pct <= 20 && !charging;
    let fill = if charging || pct > 50 { green } else if pct > 20 { amber } else { red };
    let outline = if low { red } else { ink };
    let width = (f32::from(pct) / 100.0 * 15.2).max(1.8);
    let bolt = if charging {
        format!(r#"<path d="M12.7 11.9 8.5 17.7h3.3l-1.1 4.5 4.3-6h-3.3z" fill="{ink}" stroke="{knock}" stroke-width="1" stroke-linejoin="round"/>"#)
    } else {
        String::new()
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
<g transform="translate(6.96 -.2) scale(.42)" fill="none" stroke="{ink}" stroke-width="4.3" stroke-linecap="round" stroke-linejoin="round">{glyph}</g>
<rect x="1.4" y="11.4" width="19.6" height="11.2" rx="3" fill="none" stroke="{outline}" stroke-width="2"/>
<rect x="21.6" y="14.6" width="2" height="4.8" rx="1" fill="{outline}"/>
<rect x="3.6" y="13.6" width="{width:.2}" height="6.8" rx="1.3" fill="{fill}"/>
{bolt}</svg>"#
    )
}

fn device_icon(kind: Option<Kind>, battery: Option<(u8, bool)>, light: bool) -> Image<'static> {
    render_icon(&device_svg(kind, battery, light), ICON)
}

fn render_icon(svg: &str, size: u32) -> Image<'static> {
    use resvg::{tiny_skia, usvg};
    let render = || -> Option<Vec<u8>> {
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
        let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
        let s = size as f32 / 24.0;
        resvg::render(&tree, tiny_skia::Transform::from_scale(s, s), &mut pixmap.as_mut());
        // tiny-skia stores premultiplied RGBA; the tray wants straight alpha.
        Some(pixmap.pixels().iter().flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        }).collect())
    };
    Image::new_owned(render().unwrap_or_else(|| vec![0; (size * size * 4) as usize]), size, size)
}

pub(crate) struct TrayDevice {
    pub info: DeviceInfo,
    pub connected: bool,
    pub name: String,
    pub section: Option<Tray>,
}

impl TrayDevice {
    pub fn header(&self) -> &str {
        self.section.as_ref().map_or(self.name.as_str(), |tray| tray.header.as_str())
    }

    pub fn items(&self) -> &[TrayItem] {
        self.section.as_ref().map_or(&[], |tray| tray.items.as_slice())
    }
}

pub(crate) fn devices(app: &AppHandle) -> Vec<TrayDevice> {
    app.state::<AppState>().slots.iter().map(|slot| {
        let info = slot.driver.info();
        let snap = slot.snap.lock().unwrap().clone();
        let name = snap.product.clone().filter(|name| !name.trim().is_empty()).unwrap_or_else(|| info.name.to_string());
        let section = slot.driver.tray(&snap);
        TrayDevice { info, connected: snap.connected, name, section }
    }).collect()
}

#[derive(Clone, PartialEq, Eq)]
struct IconSpec {
    id: String,
    kind: Option<Kind>,
    tooltip: String,
    battery: Option<(u8, bool)>,
}

fn desired_icons(devices: &[TrayDevice]) -> Vec<IconSpec> {
    let mut icons: Vec<_> = devices.iter().filter(|d| d.connected).map(|d| IconSpec {
        id: format!("device/{}", d.info.id),
        kind: Some(d.info.kind),
        tooltip: d.header().to_owned(),
        battery: d.section.as_ref().and_then(|tray| tray.battery),
    }).collect();
    if icons.is_empty() {
        icons.push(IconSpec { id: "main".into(), kind: None, tooltip: "Periph".into(), battery: None });
    }
    icons
}

/// Every icon is created up front, hidden, in registry order: Windows keys the user's
/// "show in taskbar" choice on (exe, icon uID), and uIDs are handed out in creation order, so
/// creating them as devices happen to connect let one device inherit another's setting.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let mut ids = vec![("main".to_string(), None, "Periph".to_string())];
    for slot in &state.slots {
        let info = slot.driver.info();
        ids.push((format!("device/{}", info.id), Some(info.kind), info.name.to_string()));
    }
    {
        let mut contents = state.tray.contents.lock().unwrap();
        for (id, kind, tooltip) in ids {
            let icon = build_icon(app, &id, kind, None, &tooltip)?;
            icon.set_visible(false)?;
            contents.icons.insert(id.clone(), (IconSpec { id, kind, tooltip, battery: None }, false));
        }
    }
    sync(app)
}

fn build_icon(app: &AppHandle, id: &str, kind: Option<Kind>, battery: Option<(u8, bool)>, tooltip: &str) -> tauri::Result<tauri::tray::TrayIcon> {
    TrayIconBuilder::with_id(id)
        .icon(device_icon(kind, battery, light_taskbar()))
        .tooltip(tooltip)
        .on_tray_icon_event(|icon, event| flyout::on_tray_event(icon.app_handle(), event))
        .build(app)
}

/// Coalesce reports from worker threads and make all icon changes on the UI thread.
pub fn refresh(app: &AppHandle, force: bool) {
    let state = app.state::<AppState>();
    if state.tray.queued.swap(true, Ordering::AcqRel) && !force {
        return;
    }
    let handle = app.clone();
    if app.run_on_main_thread(move || {
        handle.state::<AppState>().tray.queued.store(false, Ordering::Release);
        let _ = sync(&handle);
    }).is_err() {
        state.tray.queued.store(false, Ordering::Release);
    }
}

fn sync(app: &AppHandle) -> tauri::Result<()> {
    let desired = desired_icons(&devices(app));
    let state = app.state::<AppState>();
    let mut contents = state.tray.contents.lock().unwrap();
    for spec in &desired {
        if let Some(icon) = app.tray_by_id(&spec.id) {
            let previous = contents.icons.get(&spec.id);
            if previous.is_none_or(|(old, _)| old.kind != spec.kind || old.battery != spec.battery) {
                icon.set_icon(Some(device_icon(spec.kind, spec.battery, light_taskbar())))?;
            }
            if previous.is_none_or(|(old, _)| old.tooltip != spec.tooltip) {
                icon.set_tooltip(Some(&spec.tooltip))?;
            }
            if previous.is_none_or(|(_, visible)| !visible) {
                icon.set_visible(true)?;
            }
        } else {
            build_icon(app, &spec.id, spec.kind, spec.battery, &spec.tooltip)?;
        }
        contents.icons.insert(spec.id.clone(), (spec.clone(), true));
    }
    // Keep registered icons so reconnects can reuse their native tray identity.
    for (id, (_, visible)) in &mut contents.icons {
        if *visible && !desired.iter().any(|spec| &spec.id == id) {
            if let Some(icon) = app.tray_by_id(id) {
                icon.set_visible(false)?;
            }
            *visible = false;
        }
    }
    drop(contents);
    flyout::notify(app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connected_devices_get_independent_icons() {
        let device = |id: &'static str, kind, connected, battery: Option<(u8, bool)>| TrayDevice {
            info: DeviceInfo { id, kind, name: id },
            name: id.into(),
            connected,
            section: battery.map(|battery| Tray { header: id.into(), battery: Some(battery), items: Vec::new() }),
        };
        let mut devices = [
            device("mouse", Kind::Mouse, true, Some((90, false))),
            device("keyboard", Kind::Keyboard, true, None),
            device("controller", Kind::Controller, true, Some((12, true))),
        ];
        let icons = desired_icons(&devices);
        assert_eq!(icons.iter().map(|icon| icon.id.as_str()).collect::<Vec<_>>(), ["device/mouse", "device/keyboard", "device/controller"]);
        assert_eq!(icons.iter().map(|icon| icon.battery).collect::<Vec<_>>(), [Some((90, false)), None, Some((12, true))]);
        for device in &mut devices {
            device.connected = false;
        }
        assert_eq!(desired_icons(&devices)[0].id, "main");
    }

    #[test]
    fn render_tray_variants() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tray-preview");
        std::fs::create_dir_all(&dir).unwrap();
        for (theme, light) in [("dark", false), ("light", true)] {
            let variants = [
                ("periph", None, None),
                ("mouse", Some(Kind::Mouse), Some((90, false))),
                ("keyboard", Some(Kind::Keyboard), None),
                ("controller-low", Some(Kind::Controller), Some((12, false))),
                ("controller-charging", Some(Kind::Controller), Some((60, true))),
                ("mouse-half", Some(Kind::Mouse), Some((45, false))),
            ];
            let mut rendered = Vec::new();
            for (name, kind, battery) in variants {
                let svg = device_svg(kind, battery, light);
                std::fs::write(dir.join(format!("device-{theme}-{name}.svg")), &svg).unwrap();
                let image = render_icon(&svg, ICON);
                let rgba = image.rgba();
                assert_eq!(rgba.len(), (ICON * ICON * 4) as usize);
                assert!(rgba.chunks_exact(4).any(|p| p[3] > 0), "{name} is empty");
                assert!(rendered.iter().all(|previous: &Vec<u8>| previous.as_slice() != rgba), "{name} is not distinct");
                rendered.push(rgba.to_vec());
                std::fs::write(dir.join(format!("device-{theme}-{name}.rgba")), rgba).unwrap();
            }
        }
    }
}
