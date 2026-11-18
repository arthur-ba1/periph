//! Tray icon: battery gauge drawn in code, tooltip with live status.

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub enum TrayMsg {
    Toggle,
    Show,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    last: Option<(Option<u8>, String)>,
}

const S: u32 = 32;

/// Battery-shaped icon, filled according to `pct` (None = disconnected, grey).
pub fn battery_icon(pct: Option<u8>) -> Icon {
    let mut px = vec![0u8; (S * S * 4) as usize];
    let mut put = |x: u32, y: u32, c: [u8; 4]| {
        let i = ((y * S + x) * 4) as usize;
        px[i..i + 4].copy_from_slice(&c);
    };
    let white = [235, 235, 235, 255];
    // body: x 2..28, y 8..24 ; terminal: x 28..31, y 13..19
    for y in 8..24 {
        for x in 2..28 {
            if y <= 9 || y >= 22 || x <= 3 || x >= 26 {
                put(x, y, white);
            }
        }
    }
    for y in 13..19 {
        for x in 28..31 {
            put(x, y, white);
        }
    }
    let (fill, color) = match pct {
        None => (22, [120, 120, 120, 255]),
        Some(p) => {
            let c = if p > 50 { [80, 220, 100, 255] } else if p > 20 { [240, 200, 40, 255] } else { [240, 60, 50, 255] };
            ((22 * p.min(100) as u32).div_ceil(100).max(1), c)
        }
    };
    for y in 11..21 {
        for x in 5..(5 + fill) {
            put(x, y, color);
        }
    }
    Icon::from_rgba(px, S, S).expect("valid icon")
}

impl Tray {
    /// Must be called on the main (event-loop) thread.
    pub fn new(on_msg: impl Fn(TrayMsg) + Send + Sync + 'static) -> anyhow::Result<Self> {
        let open = MenuItem::new("Abrir", true, None);
        let quit = MenuItem::new("Sair", true, None);
        let menu = Menu::new();
        menu.append_items(&[&open, &PredefinedMenuItem::separator(), &quit])?;
        let (open_id, quit_id) = (open.id().clone(), quit.id().clone());

        let on_msg = std::sync::Arc::new(on_msg);
        let m = on_msg.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if e.id == open_id {
                m(TrayMsg::Show)
            } else if e.id == quit_id {
                m(TrayMsg::Quit)
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                on_msg(TrayMsg::Toggle)
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("Rapoo VT2 Max")
            .with_icon(battery_icon(None))
            .build()?;
        Ok(Self { icon, last: None })
    }

    pub fn update(&mut self, battery: Option<u8>, tooltip: String) {
        let key = (battery, tooltip);
        if self.last.as_ref() == Some(&key) {
            return;
        }
        let _ = self.icon.set_icon(Some(battery_icon(key.0)));
        let _ = self.icon.set_tooltip(Some(&key.1));
        self.last = Some(key);
    }
}
