#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! vt2-gui: window + tray configurator for the Rapoo VT2 Max.
//!
//! `vt2-gui --tray` starts hidden in the tray.

mod tray;
mod worker;

use eframe::egui::{self, Color32, RichText};
use std::sync::mpsc::{channel, Receiver, Sender};
use tray::{Tray, TrayMsg};
use vt2_proto::{ButtonAction, Status, BUTTON_NAMES};
use worker::{Cmd, Config, Event};

const POLLING: [u32; 7] = [125, 250, 500, 1000, 2000, 4000, 8000];

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Dpi,
    Performance,
    Sensor,
    Buttons,
}

struct App {
    cmd_tx: Sender<Cmd>,
    ev_rx: Receiver<Event>,
    tray_rx: Receiver<TrayMsg>,
    tray: Tray,
    visible: bool,
    tab: Tab,
    device: Option<(String, String)>,
    disconnected: Option<String>,
    config: Option<Config>,
    status: Option<Status>,
    /// Editable copy of DPI values before "Aplicar".
    dpi_edit: Vec<u16>,
    lod_edit: f32,
    debounce_edit: u8,
    key_edit: Vec<String>,
    error: Option<String>,
    busy: bool,
}

impl App {
    fn send(&mut self, c: Cmd) {
        self.busy = true;
        let _ = self.cmd_tx.send(c);
    }

    fn set_visible(&mut self, ctx: &egui::Context, v: bool) {
        self.visible = v;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(v));
        if v {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }

    fn drain(&mut self, ctx: &egui::Context) {
        while let Ok(m) = self.tray_rx.try_recv() {
            match m {
                TrayMsg::Toggle => { let v = !self.visible; self.set_visible(ctx, v) }
                TrayMsg::Show => self.set_visible(ctx, true),
                TrayMsg::Quit => std::process::exit(0),
            }
        }
        while let Ok(e) = self.ev_rx.try_recv() {
            match e {
                Event::Connected { product, serial } => {
                    self.device = Some((product, serial));
                    self.disconnected = None;
                }
                Event::Disconnected(why) => {
                    self.device = None;
                    self.config = None;
                    self.status = None;
                    self.disconnected = Some(why);
                    self.busy = false;
                }
                Event::Config(c) => {
                    self.dpi_edit = c.dpi.clone();
                    self.lod_edit = c.lod_mm;
                    self.debounce_edit = c.debounce_press;
                    self.key_edit = c
                        .buttons
                        .iter()
                        .map(|b| match b { ButtonAction::Key(k) => key_label(*k), _ => String::new() })
                        .collect();
                    self.config = Some(c);
                    self.busy = false;
                }
                Event::Status(s) => self.status = Some(s),
                Event::Error(e) => {
                    self.error = Some(e);
                    self.busy = false;
                }
            }
        }
        let tip = match (&self.status, &self.device) {
            (Some(s), _) => format!(
                "Rapoo VT2 Max\nBateria {}%\n{} DPI · {} Hz",
                s.battery, s.dpi_x, s.polling_hz.map(|h| h.to_string()).unwrap_or("?".into())
            ),
            (None, Some(_)) => "Rapoo VT2 Max\nconectado".into(),
            (None, None) => "Rapoo VT2 Max\ndesconectado".into(),
        };
        self.tray.update(self.status.map(|s| s.battery), tip);
    }
}

fn key_label(usage: u8) -> String {
    match usage {
        0x04..=0x1D => ((b'A' + usage - 0x04) as char).to_string(),
        0x1E..=0x26 => ((b'1' + usage - 0x1E) as char).to_string(),
        0x27 => "0".into(),
        0x3A..=0x45 => format!("F{}", usage - 0x39),
        0x68..=0x73 => format!("F{}", usage - 0x68 + 13),
        _ => format!("0x{usage:02X}"),
    }
}

fn parse_key(s: &str) -> Option<u8> {
    let s = s.trim().to_ascii_uppercase();
    let b = s.as_bytes();
    match b {
        [c @ b'A'..=b'Z'] => Some(0x04 + (c - b'A')),
        [c @ b'1'..=b'9'] => Some(0x1E + (c - b'1')),
        [b'0'] => Some(0x27),
        _ if s.starts_with("0X") => u8::from_str_radix(&s[2..], 16).ok(),
        _ if s.starts_with('F') => match s[1..].parse::<u8>().ok()? {
            n @ 1..=12 => Some(0x39 + n),
            n @ 13..=24 => Some(0x68 + n - 13),
            _ => None,
        },
        _ => None,
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Kind {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    DpiUp,
    DpiDown,
    Key,
    Off,
    Other,
}

impl Kind {
    const ALL: [(Kind, &'static str); 9] = [
        (Kind::Left, "Clique esquerdo"),
        (Kind::Right, "Clique direito"),
        (Kind::Middle, "Clique do meio"),
        (Kind::Back, "Voltar"),
        (Kind::Forward, "Avançar"),
        (Kind::DpiUp, "DPI +"),
        (Kind::DpiDown, "DPI -"),
        (Kind::Key, "Tecla…"),
        (Kind::Off, "Desativado"),
    ];
    fn of(a: ButtonAction) -> Kind {
        match a {
            ButtonAction::Mouse(1) => Kind::Left,
            ButtonAction::Mouse(2) => Kind::Right,
            ButtonAction::Mouse(4) => Kind::Middle,
            ButtonAction::Mouse(8) => Kind::Back,
            ButtonAction::Mouse(0x10) => Kind::Forward,
            ButtonAction::DpiUp => Kind::DpiUp,
            ButtonAction::DpiDown => Kind::DpiDown,
            ButtonAction::Key(_) => Kind::Key,
            ButtonAction::Disabled => Kind::Off,
            _ => Kind::Other,
        }
    }
    fn label(self) -> &'static str {
        Self::ALL.iter().find(|(k, _)| *k == self).map(|(_, l)| *l).unwrap_or("Outro (original)")
    }
    fn action(self) -> Option<ButtonAction> {
        Some(match self {
            Kind::Left => ButtonAction::Mouse(1),
            Kind::Right => ButtonAction::Mouse(2),
            Kind::Middle => ButtonAction::Mouse(4),
            Kind::Back => ButtonAction::Mouse(8),
            Kind::Forward => ButtonAction::Mouse(0x10),
            Kind::DpiUp => ButtonAction::DpiUp,
            Kind::DpiDown => ButtonAction::DpiDown,
            Kind::Off => ButtonAction::Disabled,
            Kind::Key | Kind::Other => return None,
        })
    }
}

const BUTTON_LABELS: [&str; 10] = [
    "Esquerdo", "Meio", "Direito", "DPI +", "DPI -", "Lateral frente", "Lateral trás", "Roda p/ cima", "Roda p/ baixo", "DIY (embaixo)",
];

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain(ctx);
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.set_visible(ctx, false);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            self.header(ui);
            ui.separator();
            if self.config.is_none() {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.spinner();
                    ui.label(self.disconnected.as_deref().map(|w| format!("Procurando o mouse… ({w})"))
                        .unwrap_or_else(|| "Lendo configuração…".into()));
                    ui.label(RichText::new("Se o A HUB estiver aberto, feche-o.").weak());
                });
                return;
            }
            ui.horizontal(|ui| {
                for (t, l) in [(Tab::Dpi, "DPI"), (Tab::Performance, "Desempenho"), (Tab::Sensor, "Sensor"), (Tab::Buttons, "Botões")] {
                    ui.selectable_value(&mut self.tab, t, RichText::new(l).size(15.0));
                }
                if self.busy {
                    ui.spinner();
                }
            });
            ui.separator();
            if let Some(e) = self.error.clone() {
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::LIGHT_RED, format!("Erro: {e}"));
                    if ui.small_button("ok").clicked() {
                        self.error = None;
                    }
                });
            }
            egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                Tab::Dpi => self.tab_dpi(ui),
                Tab::Performance => self.tab_perf(ui),
                Tab::Sensor => self.tab_sensor(ui),
                Tab::Buttons => self.tab_buttons(ui),
            });
        });
    }
}

impl App {
    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Rapoo VT2 Max");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Reler").on_hover_text("Reler do mouse").clicked() {
                    self.send(Cmd::Refresh);
                }
                match (&self.status, &self.device) {
                    (Some(s), _) => {
                        let c = if s.battery > 50 { Color32::LIGHT_GREEN } else if s.battery > 20 { Color32::YELLOW } else { Color32::LIGHT_RED };
                        ui.label(format!("{} DPI · {} Hz", s.dpi_x, s.polling_hz.map(|h| h.to_string()).unwrap_or("?".into())));
                        ui.colored_label(c, RichText::new(format!("Bateria {}%", s.battery)).strong());
                    }
                    (None, Some(_)) => { ui.label("conectado"); }
                    (None, None) => { ui.colored_label(Color32::GRAY, "desconectado"); }
                }
            });
        });
    }

    fn tab_dpi(&mut self, ui: &mut egui::Ui) {
        let c = self.config.clone().unwrap();
        ui.label("Estágios de DPI (o marcado é o ativo; os botões DPI± do mouse alternam entre eles)");
        ui.add_space(6.0);
        let mut cmds = vec![];
        egui::Grid::new("dpi").num_columns(4).spacing([12.0, 8.0]).show(ui, |ui| {
            for i in 0..c.stage_count {
                if ui.radio(c.active as usize == i, format!("Estágio {}", i + 1)).clicked() {
                    cmds.push(Cmd::ActiveStage(i as u8));
                }
                ui.add(egui::DragValue::new(&mut self.dpi_edit[i]).range(50..=26000).speed(10.0).suffix(" DPI"));
                let changed = self.dpi_edit[i] != c.dpi[i];
                if ui.add_enabled(changed, egui::Button::new("Aplicar")).clicked() {
                    let v = (self.dpi_edit[i] / 10 * 10).clamp(50, 26000);
                    cmds.push(Cmd::StageDpi(i, v));
                }
                if changed {
                    ui.label(RichText::new(format!("atual: {}", c.dpi[i])).weak());
                }
                ui.end_row();
            }
        });
        for c in cmds {
            self.send(c);
        }
    }

    fn tab_perf(&mut self, ui: &mut egui::Ui) {
        let c = self.config.clone().unwrap();
        ui.label("Polling rate");
        ui.horizontal_wrapped(|ui| {
            for hz in POLLING {
                let label = if hz >= 1000 { format!("{} kHz", hz / 1000) } else { format!("{hz} Hz") };
                if ui.selectable_label(c.polling == hz, label).clicked() && c.polling != hz {
                    self.send(Cmd::Polling(hz));
                }
            }
        });
        ui.label(RichText::new("Taxas mais altas consomem mais bateria. 4 e 8 kHz precisam do dongle compatível.").weak());
    }

    fn tab_sensor(&mut self, ui: &mut egui::Ui) {
        let c = self.config.clone().unwrap();
        let mut cmds = vec![];
        let mut toggle = |ui: &mut egui::Ui, v: bool, label: &str, hint: &str, mk: fn(bool) -> Cmd| {
            let mut x = v;
            if ui.checkbox(&mut x, label).on_hover_text(hint).changed() {
                cmds.push(mk(x));
            }
        };
        toggle(ui, c.motion_sync, "Motion sync", "Sincroniza leitura do sensor com o polling (mais suave, +~1 ms)", Cmd::MotionSync);
        toggle(ui, c.linear, "Correção linear (angle snapping)", "Corrige desvios em linhas retas", Cmd::Linear);
        toggle(ui, c.ripple, "Correção de ripple", "Reduz tremido em DPI alto", Cmd::Ripple);
        toggle(ui, c.glass, "Rastreamento em vidro", "Melhora em superfícies de vidro; desligue fora delas", Cmd::Glass);
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Lift-off distance");
            let r = ui.add(egui::Slider::new(&mut self.lod_edit, 0.7..=1.7).step_by(0.1).suffix(" mm"));
            if (r.drag_stopped() || (r.changed() && !r.dragged())) && (self.lod_edit - c.lod_mm).abs() > 0.05 {
                cmds.push(Cmd::Lod(self.lod_edit));
            }
        });
        ui.horizontal(|ui| {
            ui.label("Debounce (pressionar)");
            let r = ui.add(egui::Slider::new(&mut self.debounce_edit, 0..=32).suffix(" ms"));
            if (r.drag_stopped() || (r.changed() && !r.dragged())) && self.debounce_edit != c.debounce_press {
                cmds.push(Cmd::Debounce(self.debounce_edit));
            }
        });
        ui.label(RichText::new(format!("Debounce ao soltar: {} ms · ângulo do sensor: {}", c.debounce_release, c.angle)).weak());
        for c in cmds {
            self.send(c);
        }
    }

    fn tab_buttons(&mut self, ui: &mut egui::Ui) {
        let c = self.config.clone().unwrap();
        let mut cmds = vec![];
        egui::Grid::new("btns").num_columns(3).spacing([12.0, 8.0]).show(ui, |ui| {
            for (i, cur) in c.buttons.iter().enumerate() {
                ui.label(BUTTON_LABELS[i]).on_hover_text(BUTTON_NAMES[i]);
                let kind = Kind::of(*cur);
                let mut sel = kind;
                egui::ComboBox::from_id_salt(("btn", i)).width(150.0).selected_text(kind.label()).show_ui(ui, |ui| {
                    for (k, l) in Kind::ALL {
                        ui.selectable_value(&mut sel, k, l);
                    }
                });
                if sel != kind {
                    if let Some(a) = sel.action() {
                        cmds.push(Cmd::Button(i, a));
                    } else if sel == Kind::Key {
                        self.key_edit[i] = "F13".into();
                        cmds.push(Cmd::Button(i, ButtonAction::Key(0x68)));
                    }
                }
                if kind == Kind::Key {
                    ui.horizontal(|ui| {
                        let r = ui.add(egui::TextEdit::singleline(&mut self.key_edit[i]).desired_width(60.0).hint_text("A, F13…"));
                        let enter = r.lost_focus() && ui.input(|inp| inp.key_pressed(egui::Key::Enter));
                        if enter || ui.small_button("ok").clicked() {
                            match parse_key(&self.key_edit[i]) {
                                Some(k) => cmds.push(Cmd::Button(i, ButtonAction::Key(k))),
                                None => self.error = Some(format!("tecla inválida: {}", self.key_edit[i])),
                            }
                        }
                    });
                } else {
                    ui.label("");
                }
                ui.end_row();
            }
        });
        ui.label(RichText::new("Teclas: A–Z, 0–9, F1–F24 ou código HID 0xNN.").weak());
        for c in cmds {
            self.send(c);
        }
    }
}

fn main() -> eframe::Result<()> {
    let start_hidden = std::env::args().any(|a| a == "--tray");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rapoo VT2 Max")
            .with_inner_size([520.0, 460.0])
            .with_min_inner_size([420.0, 320.0])
            .with_visible(!start_hidden),
        ..Default::default()
    };
    eframe::run_native(
        "rapoo-vt2",
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let (cmd_tx, cmd_rx) = channel();
            let (ev_tx, ev_rx) = channel();
            let wctx = ctx.clone();
            std::thread::spawn(move || worker::run(cmd_rx, ev_tx, move || wctx.request_repaint()));

            let (tray_tx, tray_rx) = channel();
            let tctx = ctx.clone();
            let tray = Tray::new(move |m| {
                let _ = tray_tx.send(m);
                tctx.request_repaint();
            })?;

            Ok(Box::new(App {
                cmd_tx,
                ev_rx,
                tray_rx,
                tray,
                visible: !start_hidden,
                tab: Tab::Dpi,
                device: None,
                disconnected: None,
                config: None,
                status: None,
                dpi_edit: vec![],
                lod_edit: 1.1,
                debounce_edit: 4,
                key_edit: vec![String::new(); BUTTON_NAMES.len()],
                error: None,
                busy: false,
            }))
        }),
    )
}
