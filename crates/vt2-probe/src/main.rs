//! vt2-probe: READ-ONLY discovery of Rapoo HID interfaces.
//!
//! Usage:
//!   vt2-probe                 list interfaces + report descriptors
//!   vt2-probe --listen 10     also print input reports from vendor interfaces for 10 s
//!   vt2-probe --vid 24ae      vendor id in hex (default 24ae)
//!   vt2-probe --vid 054c --ds-features
//!                             read-only dump of the DualSense feature reports a virtual
//!                             controller must answer (0x05 calibration, 0x09 MAC, 0x20 firmware
//!                             info, 0x22 BT patch). Uses GET_FEATURE only; nothing is written.
//!
//! This tool never sends output or feature reports to the device.

mod descriptor;

use anyhow::Result;
use hidapi::{DeviceInfo, HidApi};
use std::time::{Duration, Instant};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

fn is_vendor(d: &DeviceInfo) -> bool {
    d.usage_page() >= 0xFF00
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1));
    let vid = u16::from_str_radix(arg("--vid").map(String::as_str).unwrap_or("24ae"), 16)?;
    let listen = arg("--listen").map(|s| s.parse::<u64>()).transpose()?;

    let api = HidApi::new()?;

    if args.iter().any(|a| a == "--ds-features") {
        // GET_FEATURE reads only, on the standard gamepad collection (usage page 0x01/usage 0x05).
        for d in api.device_list().filter(|d| {
            d.vendor_id() == vid && d.product_id() == 0x0CE6 && d.usage_page() == 0x01 && d.usage() == 0x05
        }) {
            println!("## {}  serial={:?}", d.path().to_string_lossy(), d.serial_number().unwrap_or(""));
            let dev = match d.open_device(&api) {
                Ok(h) => h,
                Err(e) => {
                    println!("  (could not open: {e})\n");
                    continue;
                }
            };
            for (id, len) in [(0x05u8, 41usize), (0x09, 20), (0x20, 64), (0x22, 64)] {
                let mut buf = vec![0u8; len];
                buf[0] = id;
                match dev.get_feature_report(&mut buf) {
                    Ok(n) => println!("  feature 0x{id:02X} ({n} bytes): {}", hex(&buf[..n])),
                    Err(e) => println!("  feature 0x{id:02X}: error: {e}"),
                }
            }
            println!();
        }
        return Ok(());
    }

    let mut devs: Vec<&DeviceInfo> = api.device_list().filter(|d| d.vendor_id() == vid).collect();
    devs.sort_by_key(|d| (d.product_id(), d.interface_number(), d.usage_page(), d.usage()));

    if devs.is_empty() {
        println!("No HID devices with VID {vid:04X} found.");
        return Ok(());
    }

    println!("# HID interfaces for VID {vid:04X}\n");
    for d in &devs {
        println!(
            "## PID {:04X}  MI_{:02}  usage_page=0x{:04X} usage=0x{:04X}{}",
            d.product_id(),
            d.interface_number(),
            d.usage_page(),
            d.usage(),
            if is_vendor(d) { "  [VENDOR]" } else { "" }
        );
        println!(
            "  product={:?} manufacturer={:?} serial={:?}",
            d.product_string().unwrap_or(""),
            d.manufacturer_string().unwrap_or(""),
            d.serial_number().unwrap_or("")
        );
        println!("  path={}", d.path().to_string_lossy());

        match d.open_device(&api) {
            Err(e) => println!("  (could not open: {e} — normal for mouse/keyboard collections owned by Windows)\n"),
            Ok(dev) => {
                let mut buf = [0u8; 4096];
                match dev.get_report_descriptor(&mut buf) {
                    Ok(n) => {
                        let desc = &buf[..n];
                        println!("  report descriptor ({n} bytes): {}", hex(desc));
                        print!("{}", descriptor::parse(desc).to_text());
                    }
                    Err(e) => println!("  (report descriptor unavailable: {e})"),
                }
                println!();
            }
        }
    }

    if let Some(secs) = listen {
        println!("# Listening {secs}s on vendor interfaces (move/click the mouse, wait for battery updates)\n");
        let opened: Vec<_> = devs
            .iter()
            .filter(|d| is_vendor(d))
            .filter_map(|d| d.open_device(&api).ok().map(|h| (*d, h)))
            .collect();
        for (_, h) in &opened {
            h.set_blocking_mode(false)?;
        }
        let end = Instant::now() + Duration::from_secs(secs);
        let mut buf = [0u8; 1024];
        while Instant::now() < end {
            for (d, h) in &opened {
                if let Ok(n) = h.read(&mut buf) {
                    if n > 0 {
                        println!(
                            "[MI_{:02} 0x{:04X}/0x{:04X}] {}",
                            d.interface_number(),
                            d.usage_page(),
                            d.usage(),
                            hex(&buf[..n])
                        );
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    Ok(())
}
