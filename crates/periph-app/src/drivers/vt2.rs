//! Rapoo VT2 Max driver (USB 24AE:1423). Protocol: vt2-proto, transport: vt2-device.
//! Frontend: app/src/devices/vt2/.

use periph_core::{self as core, serde_json::{self, Value}, Connected, DeviceInfo, Driver, Kind, Session, Snapshot, Tray, TrayItem};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vt2_device::Vt2;
use vt2_proto::{self as p, addr, ButtonAction};

/// Commands from the UI. JSON: `{"type":"polling","value":1000}`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Cmd {
    Refresh,
    Polling(u32),
    StageDpi((usize, u16)),
    ActiveStage(u8),
    Lod(f32),
    MotionSync(bool),
    Glass(bool),
    Linear(bool),
    Ripple(bool),
    Debounce(u8),
    Button((usize, ButtonAction)),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub polling: u32,
    pub dpi: Vec<u16>,
    pub stage_count: usize,
    pub active: u8,
    pub lod_mm: f32,
    pub motion_sync: bool,
    pub linear: bool,
    pub ripple: bool,
    pub glass: bool,
    pub angle: i8,
    pub debounce_press: u8,
    pub debounce_release: u8,
    pub buttons: Vec<ButtonAction>,
}

fn read_config(d: &Vt2) -> anyhow::Result<Config> {
    let (xs, _ys, last, active) = d.dpi_stages()?;
    let flags = d.get_byte(addr::CORRECTION_FLAGS)?;
    let sensor = d.read(addr::DEBOUNCE_PRESS, 6)?; // 0x08C0..0x08C5
    let raw = d.read(addr::BUTTONS, 4 * addr::BUTTON_COUNT)?;
    let buttons = raw.chunks(4).map(|c| ButtonAction::decode([c[0], c[1], c[2], c[3]])).collect();
    let lm = d.read(addr::LOD, 2)?; // 0x0884 LOD, 0x0885 motion sync
    Ok(Config {
        polling: d.polling()?,
        stage_count: (last as usize + 1).min(xs.len()),
        dpi: xs,
        active,
        lod_mm: (p::lod_mm(lm[0]) * 10.0).round() / 10.0,
        motion_sync: lm[1] != 0,
        linear: flags & 1 == 0,
        ripple: flags & 2 == 0,
        glass: sensor[5] != 0,
        angle: sensor[4] as i8,
        debounce_press: sensor[0],
        debounce_release: sensor[1],
        buttons,
    })
}

fn apply(d: &Vt2, cmd: Cmd) -> anyhow::Result<()> {
    match cmd {
        Cmd::Refresh => Ok(()),
        Cmd::Polling(hz) => d.set_polling(hz),
        Cmd::StageDpi((s, v)) => d.set_stage_dpi(s, v),
        Cmd::ActiveStage(s) => d.set_active_stage(s),
        Cmd::Lod(mm) => d.set_byte(addr::LOD, p::lod_raw(mm)?),
        Cmd::MotionSync(on) => d.set_byte(addr::MOTION_SYNC, on as u8),
        Cmd::Glass(on) => d.set_byte(addr::GLASS, on as u8),
        Cmd::Linear(on) | Cmd::Ripple(on) => {
            let bit = if matches!(cmd, Cmd::Linear(_)) { 1 } else { 2 };
            let cur = d.get_byte(addr::CORRECTION_FLAGS)?;
            d.set_byte(addr::CORRECTION_FLAGS, if on { cur & !bit } else { cur | bit })
        }
        Cmd::Debounce(ms) => d.set_byte(addr::DEBOUNCE_PRESS, p::check_debounce(ms)?),
        Cmd::Button((i, a)) => {
            // Only allow action shapes seen from the official app.
            match a {
                ButtonAction::Raw(_) | ButtonAction::Wheel(_) => anyhow::bail!("ação não suportada"),
                ButtonAction::Key(k) if !(0x04..=0x73).contains(&k) => anyhow::bail!("tecla fora do intervalo"),
                _ => {}
            }
            d.set_button(i, a)
        }
    }
}

struct Vt2Session(Vt2);

impl Session for Vt2Session {
    fn read_config(&mut self) -> anyhow::Result<Value> {
        Ok(serde_json::to_value(read_config(&self.0)?)?)
    }
    fn apply(&mut self, cmd: Value) -> anyhow::Result<()> {
        apply(&self.0, core::parse(cmd)?)
    }
    fn poll(&mut self) -> anyhow::Result<Option<Value>> {
        Ok(self.0.status(Duration::from_millis(250))?.map(|s| serde_json::to_value(s)).transpose()?)
    }
}

pub struct Vt2Driver;

const POLLING: [u32; 7] = [125, 250, 500, 1000, 2000, 4000, 8000];

fn fmt_hz(hz: u32) -> String {
    if hz >= 1000 { format!("{} kHz", hz / 1000) } else { format!("{hz} Hz") }
}

impl Driver for Vt2Driver {
    fn info(&self) -> DeviceInfo {
        DeviceInfo { id: "vt2", name: "Rapoo VT2 Max", kind: Kind::Mouse }
    }

    fn open(&self) -> anyhow::Result<(Box<dyn Session>, Connected)> {
        let d = Vt2::open()?;
        let info = Connected { product: d.product.clone(), serial: d.serial.clone() };
        Ok((Box::new(Vt2Session(d)), info))
    }

    fn tray(&self, s: &Snapshot) -> Option<Tray> {
        let status: Option<p::Status> = s.status.clone().and_then(|v| serde_json::from_value(v).ok());
        let header = match (&status, s.connected) {
            (Some(st), _) => format!(
                "VT2 Max  ·  {}  ·  bateria {}%{}  ·  {} DPI",
                if st.wired { "cabo" } else { "sem fio" },
                st.battery,
                if st.charging { " (carregando)" } else { "" },
                st.dpi_x,
            ),
            (None, true) => "VT2 Max  ·  conectado".into(),
            (None, false) => "VT2 Max  ·  desconectado".into(),
        };
        let mut items = Vec::new();
        if let Some(c) = s.config.clone().and_then(|v| serde_json::from_value::<Config>(v).ok()) {
            items.push(TrayItem::Submenu {
                label: format!("DPI  ·  {}", c.dpi[c.active as usize]),
                items: (0..c.stage_count)
                    .map(|i| TrayItem::Check {
                        id: format!("stage:{i}"),
                        label: format!("Estágio {}  ·  {} DPI", i + 1, c.dpi[i]),
                        checked: c.active as usize == i,
                    })
                    .collect(),
            });
            items.push(TrayItem::Submenu {
                label: format!("Polling rate  ·  {}", fmt_hz(c.polling)),
                items: POLLING.iter().map(|&hz| TrayItem::Check { id: format!("poll:{hz}"), label: fmt_hz(hz), checked: c.polling == hz }).collect(),
            });
            items.push(TrayItem::Check { id: "motion".into(), label: "Motion Sync".into(), checked: c.motion_sync });
            items.push(TrayItem::Check { id: "glass".into(), label: "Modo vidro".into(), checked: c.glass });
            items.push(TrayItem::Action { id: "refresh".into(), label: "Reler configuração do mouse".into() });
        }
        Some(Tray { header, battery: status.map(|s| (s.battery, s.charging)), items })
    }

    fn tray_command(&self, id: &str, s: &Snapshot) -> Option<Value> {
        let c: Option<Config> = s.config.clone().and_then(|v| serde_json::from_value(v).ok());
        let cmd = |t: &str, v: Value| Some(serde_json::json!({ "type": t, "value": v }));
        match id {
            "refresh" => Some(serde_json::json!({ "type": "refresh" })),
            "motion" => cmd("motionSync", (!c?.motion_sync).into()),
            "glass" => cmd("glass", (!c?.glass).into()),
            _ => {
                if let Some(i) = id.strip_prefix("stage:").and_then(|v| v.parse::<u8>().ok()) {
                    cmd("activeStage", i.into())
                } else if let Some(hz) = id.strip_prefix("poll:").and_then(|v| v.parse::<u32>().ok()) {
                    cmd("polling", hz.into())
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_commands_parse_as_cmd() {
        let snap = Snapshot {
            connected: true,
            config: Some(serde_json::json!({
                "polling": 1000, "dpi": [800, 1600], "stageCount": 2, "active": 0, "lodMm": 1.0,
                "motionSync": true, "linear": true, "ripple": false, "glass": false, "angle": 0,
                "debouncePress": 4, "debounceRelease": 4, "buttons": []
            })),
            ..Default::default()
        };
        for id in ["refresh", "motion", "glass", "stage:1", "poll:2000"] {
            let v = Vt2Driver.tray_command(id, &snap).unwrap_or_else(|| panic!("{id}"));
            core::parse::<Cmd>(v).unwrap_or_else(|e| panic!("{id}: {e}"));
        }
        assert_eq!(Vt2Driver.tray(&snap).unwrap().items.len(), 5);
    }
}
