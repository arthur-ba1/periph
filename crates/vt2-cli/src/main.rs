use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use std::time::Duration;
use vt2_device::Vt2;
use vt2_proto::{self as p, addr, ButtonAction, BUTTON_NAMES};

#[derive(Parser)]
#[command(name = "vt2", about = "Configurador do Rapoo VT2 Max")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Mostra tudo: status, DPI, polling, sensor, botões
    Info,
    /// Bateria, DPI e polling atuais (report de status)
    Status,
    /// Polling rate: sem argumento lê; com argumento grava (125..8000)
    Polling { hz: Option<u32> },
    /// DPI: `vt2 dpi` lista; `vt2 dpi <estágio 1-7> <valor>` grava; `vt2 dpi active <estágio>`
    Dpi { args: Vec<String> },
    /// Lift-off distance em mm (0.7..1.7)
    Lod { mm: Option<f32> },
    /// Motion sync on/off
    MotionSync { state: Option<OnOff> },
    /// Glass tracking on/off
    Glass { state: Option<OnOff> },
    /// Linear correction on/off
    Linear { state: Option<OnOff> },
    /// Ripple correction on/off
    Ripple { state: Option<OnOff> },
    /// Debounce ao pressionar (ms)
    Debounce { ms: Option<u8> },
    /// Botões: `vt2 button` lista; `vt2 button back middle|left|right|back|forward|key:A|dpi+|dpi-|off`
    Button { name: Option<String>, action: Option<String> },
    /// Lê memória crua (hex): `vt2 read 0880 8`
    Read { addr: String, len: usize },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OnOff {
    On,
    Off,
}

fn onoff(b: bool) -> &'static str {
    if b { "on" } else { "off" }
}

fn describe(a: ButtonAction) -> String {
    match a {
        ButtonAction::Mouse(1) => "left".into(),
        ButtonAction::Mouse(2) => "right".into(),
        ButtonAction::Mouse(4) => "middle".into(),
        ButtonAction::Mouse(8) => "back".into(),
        ButtonAction::Mouse(0x10) => "forward".into(),
        ButtonAction::Key(k) => format!("key:{}", key_name(k)),
        ButtonAction::DpiUp => "dpi+".into(),
        ButtonAction::DpiDown => "dpi-".into(),
        ButtonAction::Disabled => "off".into(),
        other => format!("{other:?}"),
    }
}

fn key_name(usage: u8) -> String {
    match usage {
        0x04..=0x1D => ((b'A' + usage - 0x04) as char).to_string(),
        0x1E..=0x26 => ((b'1' + usage - 0x1E) as char).to_string(),
        0x27 => "0".into(),
        0x3A..=0x45 => format!("F{}", usage - 0x39),
        _ => format!("0x{usage:02X}"),
    }
}

fn parse_action(s: &str) -> Result<ButtonAction> {
    let s = s.to_ascii_lowercase();
    Ok(match s.as_str() {
        "left" => ButtonAction::Mouse(1),
        "right" => ButtonAction::Mouse(2),
        "middle" => ButtonAction::Mouse(4),
        "back" => ButtonAction::Mouse(8),
        "forward" => ButtonAction::Mouse(0x10),
        "dpi+" => ButtonAction::DpiUp,
        "dpi-" => ButtonAction::DpiDown,
        "off" => ButtonAction::Disabled,
        k if k.starts_with("key:") => {
            let key = &k[4..];
            let b = key.as_bytes();
            let usage = match (b.len(), b.first()) {
                (1, Some(c @ b'a'..=b'z')) => 0x04 + (c - b'a'),
                (1, Some(c @ b'1'..=b'9')) => 0x1E + (c - b'1'),
                (1, Some(b'0')) => 0x27,
                _ if key.starts_with('f') => {
                    let n: u8 = key[1..].parse()?;
                    if !(1..=12).contains(&n) { bail!("F1..F12") }
                    0x39 + n
                }
                _ if key.starts_with("0x") => u8::from_str_radix(&key[2..], 16)?,
                _ => bail!("tecla não suportada: {key} (use a-z, 0-9, f1-f12 ou 0xNN)"),
            };
            ButtonAction::Key(usage)
        }
        _ => bail!("ação desconhecida: {s}"),
    })
}

fn button_index(name: &str) -> Result<usize> {
    BUTTON_NAMES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name))
        .ok_or_else(|| anyhow::anyhow!("botão desconhecido: {name} ({})", BUTTON_NAMES.join(", ")))
}

fn toggle(dev: &Vt2, address: u16, state: Option<OnOff>, label: &str) -> Result<()> {
    if let Some(s) = state {
        dev.set_byte(address, matches!(s, OnOff::On) as u8)?;
    }
    println!("{label}: {}", onoff(dev.get_byte(address)? != 0));
    Ok(())
}

/// bit set in CORRECTION_FLAGS means the feature is DISABLED.
fn correction(dev: &Vt2, bit: u8, state: Option<OnOff>, label: &str) -> Result<()> {
    let cur = dev.get_byte(addr::CORRECTION_FLAGS)?;
    if let Some(s) = state {
        let v = match s { OnOff::On => cur & !bit, OnOff::Off => cur | bit };
        dev.set_byte(addr::CORRECTION_FLAGS, v)?;
    }
    println!("{label}: {}", onoff(dev.get_byte(addr::CORRECTION_FLAGS)? & bit == 0));
    Ok(())
}

fn print_dpi(dev: &Vt2) -> Result<()> {
    let (xs, ys, last, active) = dev.dpi_stages()?;
    for i in 0..=(last as usize).min(xs.len() - 1) {
        let mark = if i == active as usize { "*" } else { " " };
        if xs[i] == ys[i] {
            println!(" {mark} estágio {}: {}", i + 1, xs[i]);
        } else {
            println!(" {mark} estágio {}: X {} / Y {}", i + 1, xs[i], ys[i]);
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let dev = Vt2::open()?;

    match cli.cmd {
        Cmd::Status => match dev.status(Duration::from_secs(4))? {
            Some(s) => println!(
                "bateria {}%  DPI {}x{} (estágio {})  polling {} Hz",
                s.battery, s.dpi_x, s.dpi_y, s.dpi_stage + 1,
                s.polling_hz.map(|h| h.to_string()).unwrap_or("?".into())
            ),
            None => println!("nenhum report de status recebido"),
        },
        Cmd::Info => {
            println!("{} (serial {})", dev.product, dev.serial);
            if let Some(s) = dev.status(Duration::from_secs(4))? {
                println!("bateria: {}%", s.battery);
            }
            println!("polling: {} Hz", dev.polling()?);
            println!("DPI:");
            print_dpi(&dev)?;
            println!("lift-off: {:.1} mm", p::lod_mm(dev.get_byte(addr::LOD)?));
            println!("motion sync: {}", onoff(dev.get_byte(addr::MOTION_SYNC)? != 0));
            let flags = dev.get_byte(addr::CORRECTION_FLAGS)?;
            println!("linear correction: {}", onoff(flags & 1 == 0));
            println!("ripple correction: {}", onoff(flags & 2 == 0));
            println!("glass tracking: {}", onoff(dev.get_byte(addr::GLASS)? != 0));
            println!("ângulo do sensor: {}", dev.get_byte(addr::SENSOR_ANGLE)? as i8);
            let db = dev.read(addr::DEBOUNCE_PRESS, 2)?;
            println!("debounce: press {} ms / release {} ms", db[0], db[1]);
            println!("botões:");
            for (i, n) in BUTTON_NAMES.iter().enumerate() {
                println!("  {n:<11} {}", describe(dev.button(i)?));
            }
        }
        Cmd::Polling { hz } => {
            if let Some(hz) = hz {
                dev.set_polling(hz)?;
            }
            println!("polling: {} Hz", dev.polling()?);
        }
        Cmd::Dpi { args } => match args.as_slice() {
            [] => print_dpi(&dev)?,
            [a, s] if a == "active" => {
                let s: u8 = s.parse()?;
                if !(1..=7).contains(&s) { bail!("estágio 1..7") }
                dev.set_active_stage(s - 1)?;
                print_dpi(&dev)?;
            }
            [s, v] => {
                let s: usize = s.parse()?;
                if !(1..=7).contains(&s) { bail!("estágio 1..7") }
                dev.set_stage_dpi(s - 1, v.parse()?)?;
                print_dpi(&dev)?;
            }
            _ => bail!("uso: vt2 dpi | vt2 dpi <estágio> <valor> | vt2 dpi active <estágio>"),
        },
        Cmd::Lod { mm } => {
            if let Some(mm) = mm {
                dev.set_byte(addr::LOD, p::lod_raw(mm)?)?;
            }
            println!("lift-off: {:.1} mm", p::lod_mm(dev.get_byte(addr::LOD)?));
        }
        Cmd::MotionSync { state } => toggle(&dev, addr::MOTION_SYNC, state, "motion sync")?,
        Cmd::Glass { state } => toggle(&dev, addr::GLASS, state, "glass tracking")?,
        Cmd::Linear { state } => correction(&dev, 1, state, "linear correction")?,
        Cmd::Ripple { state } => correction(&dev, 2, state, "ripple correction")?,
        Cmd::Debounce { ms } => {
            if let Some(ms) = ms {
                dev.set_byte(addr::DEBOUNCE_PRESS, p::check_debounce(ms)?)?;
            }
            println!("debounce press: {} ms", dev.get_byte(addr::DEBOUNCE_PRESS)?);
        }
        Cmd::Button { name, action } => match (name, action) {
            (None, _) => {
                for (i, n) in BUTTON_NAMES.iter().enumerate() {
                    println!("{n:<11} {}", describe(dev.button(i)?));
                }
            }
            (Some(n), a) => {
                let i = button_index(&n)?;
                if let Some(a) = a {
                    dev.set_button(i, parse_action(&a)?)?;
                }
                println!("{n}: {}", describe(dev.button(i)?));
            }
        },
        Cmd::Read { addr: a, len } => {
            let a = u16::from_str_radix(a.trim_start_matches("0x"), 16)?;
            let data = dev.read(a, len)?;
            for (i, row) in data.chunks(16).enumerate() {
                let hex: Vec<String> = row.iter().map(|b| format!("{b:02X}")).collect();
                println!("{:04X}: {}", a as usize + i * 16, hex.join(" "));
            }
        }
    }
    Ok(())
}
