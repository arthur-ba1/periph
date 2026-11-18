//! Device worker thread: owns the HID handle, applies commands, polls status.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;
use vt2_device::Vt2;
use vt2_proto::{self as p, addr, ButtonAction, Status};

#[derive(Debug, Clone)]
pub enum Cmd {
    Refresh,
    Polling(u32),
    StageDpi(usize, u16),
    ActiveStage(u8),
    Lod(f32),
    MotionSync(bool),
    Glass(bool),
    Linear(bool),
    Ripple(bool),
    Debounce(u8),
    Button(usize, ButtonAction),
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone)]
pub enum Event {
    Connected { product: String, serial: String },
    Disconnected(String),
    Config(Config),
    Status(Status),
    Error(String),
}

fn read_config(d: &Vt2) -> anyhow::Result<Config> {
    let (xs, _ys, last, active) = d.dpi_stages()?;
    let flags = d.get_byte(addr::CORRECTION_FLAGS)?;
    let sensor = d.read(addr::DEBOUNCE_PRESS, 6)?; // C0..C5
    let mut buttons = Vec::with_capacity(addr::BUTTON_COUNT);
    let raw = d.read(addr::BUTTONS, 4 * addr::BUTTON_COUNT)?;
    for c in raw.chunks(4) {
        buttons.push(ButtonAction::decode([c[0], c[1], c[2], c[3]]));
    }
    let lm = d.read(addr::LOD, 2)?; // 84, 85
    Ok(Config {
        polling: d.polling()?,
        stage_count: (last as usize + 1).min(xs.len()),
        dpi: xs,
        active,
        lod_mm: p::lod_mm(lm[0]),
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
        Cmd::StageDpi(s, v) => d.set_stage_dpi(s, v),
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
        Cmd::Button(i, a) => d.set_button(i, a),
    }
}

/// Runs forever. `notify` is called after each event so the UI can repaint.
pub fn run(rx: Receiver<Cmd>, tx: Sender<Event>, notify: impl Fn()) {
    let log_path = std::env::var_os("VT2_LOG");
    let send = |e: Event| {
        if let Some(p) = &log_path {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
                let _ = writeln!(f, "{:?}", e);
            }
        }
        let _ = tx.send(e);
        notify();
    };
    loop {
        let dev = match Vt2::open() {
            Ok(d) => d,
            Err(e) => {
                send(Event::Disconnected(e.to_string()));
                // drain commands while disconnected, retry every 2 s
                let _ = rx.recv_timeout(Duration::from_secs(2));
                continue;
            }
        };
        send(Event::Connected { product: dev.product.clone(), serial: dev.serial.clone() });
        match read_config(&dev) {
            Ok(c) => send(Event::Config(c)),
            Err(e) => {
                send(Event::Disconnected(e.to_string()));
                continue;
            }
        }
        loop {
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok(cmd) => {
                    if let Err(e) = apply(&dev, cmd) {
                        send(Event::Error(e.to_string()));
                    }
                    match read_config(&dev) {
                        Ok(c) => send(Event::Config(c)),
                        Err(e) => {
                            send(Event::Disconnected(e.to_string()));
                            break;
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => match dev.status(Duration::from_millis(250)) {
                    Ok(Some(s)) => send(Event::Status(s)),
                    Ok(None) => {}
                    Err(e) => {
                        send(Event::Disconnected(e.to_string()));
                        break;
                    }
                },
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }
}
