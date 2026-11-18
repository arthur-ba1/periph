//! via-probe: send ONLY read (GET) commands of the VIA protocol to a QMK/VIA keyboard.
//! Never sends set/save/reset commands.
//!
//! Usage: via-probe [--vid fffe] [--pid 000b]

use anyhow::{bail, Context, Result};
use hidapi::{HidApi, HidDevice};

const ID_GET_PROTOCOL_VERSION: u8 = 0x01;
const ID_GET_KEYBOARD_VALUE: u8 = 0x02;
const ID_DYNAMIC_KEYMAP_GET_KEYCODE: u8 = 0x04;
const ID_CUSTOM_GET_VALUE: u8 = 0x08;
const ID_DYNAMIC_KEYMAP_GET_LAYER_COUNT: u8 = 0x11;
const ID_DYNAMIC_KEYMAP_GET_BUFFER: u8 = 0x12;
const ID_DYNAMIC_KEYMAP_MACRO_GET_COUNT: u8 = 0x0C;
const ID_UNHANDLED: u8 = 0xFF;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

fn query(dev: &HidDevice, payload: &[u8]) -> Result<[u8; 32]> {
    let mut buf = [0u8; 32];
    while dev.read_timeout(&mut buf, 0)? > 0 {} // drop replies meant for other VIA clients
    let mut out = [0u8; 33]; // report id 0 + 32 bytes
    out[1..1 + payload.len()].copy_from_slice(payload);
    dev.write(&out).context("write")?;
    let echo = payload.len().min(4);
    for _ in 0..16 {
        let n = dev.read_timeout(&mut buf, 500)?;
        if n == 0 {
            bail!("no reply to {:02X?}", payload);
        }
        if buf[0] == ID_UNHANDLED && buf[1..echo] == payload[1..echo] {
            bail!("command 0x{:02X} unhandled by firmware", payload[0]);
        }
        if buf[..echo] == payload[..echo] {
            return Ok(buf);
        }
    }
    bail!("unexpected reply to {:02X?}", payload)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |n: &str| args.iter().position(|a| a == n).and_then(|i| args.get(i + 1)).cloned();
    let vid = u16::from_str_radix(&arg("--vid").unwrap_or("fffe".into()), 16)?;
    let pid = u16::from_str_radix(&arg("--pid").unwrap_or("000b".into()), 16)?;

    let api = HidApi::new()?;
    let info = api
        .device_list()
        .find(|d| d.vendor_id() == vid && d.product_id() == pid && d.usage_page() == 0xFF60 && d.usage() == 0x61)
        .context("VIA raw HID interface (0xFF60/0x61) not found")?;
    println!("{} / {}", info.manufacturer_string().unwrap_or("?"), info.product_string().unwrap_or("?"));
    let dev = info.open_device(&api)?;

    let r = query(&dev, &[ID_GET_PROTOCOL_VERSION])?;
    println!("VIA protocol version: 0x{:04X}", u16::from_be_bytes([r[1], r[2]]));

    if let Ok(r) = query(&dev, &[ID_GET_KEYBOARD_VALUE, 0x01]) {
        println!("uptime: {} ms", u32::from_be_bytes([r[2], r[3], r[4], r[5]]));
    }
    if let Ok(r) = query(&dev, &[ID_GET_KEYBOARD_VALUE, 0x02]) {
        println!("layout options: {}", hex(&r[2..6]));
    }

    let layers = query(&dev, &[ID_DYNAMIC_KEYMAP_GET_LAYER_COUNT])?[1];
    println!("layers: {layers}");
    if let Ok(r) = query(&dev, &[ID_DYNAMIC_KEYMAP_MACRO_GET_COUNT]) {
        println!("macros: {}", r[1]);
    }

    // RGB matrix (channel 3): brightness=1, effect=2, speed=3, color=4 (hue, sat)
    for (id, name) in [(1u8, "brightness"), (2, "effect"), (3, "speed"), (4, "color h,s")] {
        match query(&dev, &[ID_CUSTOM_GET_VALUE, 3, id]) {
            Ok(r) => println!("rgb {name}: {}", hex(&r[3..5])),
            Err(e) => println!("rgb {name}: {e}"),
        }
    }

    // keymap layer 0..layers via buffer (rows 6 x cols 16 x 2 bytes)
    let (rows, cols) = (6usize, 16usize);
    let total = layers as usize * rows * cols * 2;
    let mut buf = Vec::with_capacity(total);
    let mut off = 0usize;
    while off < total {
        let size = (total - off).min(28) as u8;
        let r = query(&dev, &[ID_DYNAMIC_KEYMAP_GET_BUFFER, (off >> 8) as u8, off as u8, size])?;
        buf.extend_from_slice(&r[4..4 + size as usize]);
        off += size as usize;
    }
    for l in 0..layers as usize {
        println!("\nlayer {l}:");
        for row in 0..rows {
            let codes: Vec<String> = (0..cols)
                .map(|c| {
                    let i = ((l * rows + row) * cols + c) * 2;
                    format!("{:04X}", u16::from_be_bytes([buf[i], buf[i + 1]]))
                })
                .collect();
            println!("  r{row}: {}", codes.join(" "));
        }
    }
    // encoder 0: id_dynamic_keymap_get_encoder (0x14) [layer, encoder, clockwise]
    for l in 0..layers {
        let ccw = query(&dev, &[0x14, l, 0, 0]);
        let cw = query(&dev, &[0x14, l, 0, 1]);
        match (ccw, cw) {
            (Ok(a), Ok(b)) => println!("encoder L{l}: ccw {:04X}  cw {:04X}  raw {}", u16::from_be_bytes([a[4], a[5]]), u16::from_be_bytes([b[4], b[5]]), hex(&b[..8])),
            (Err(e), _) | (_, Err(e)) => { println!("encoder L{l}: {e}"); break }
        }
    }
    // sanity: single keycode read must match the buffer
    let r = query(&dev, &[ID_DYNAMIC_KEYMAP_GET_KEYCODE, 0, 0, 0])?;
    println!("\nkeycode L0 r0 c0 via 0x04: {:04X}", u16::from_be_bytes([r[4], r[5]]));
    Ok(())
}
