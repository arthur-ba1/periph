//! Read-only HID discovery for package authors: `periph-pack probe`.
//! Lists devices, their interfaces (top-level collections) and report layouts, and can listen
//! to what the device sends on its own. It never sends anything to a device.

use crate::descriptor;
use anyhow::Result;
use hidapi::{DeviceInfo, HidApi};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

/// One line per USB device (VID:PID), with how many vendor-defined interfaces it has.
pub fn list(api: &HidApi) {
    let mut by: BTreeMap<(u16, u16), (String, String, usize, usize)> = BTreeMap::new();
    for d in api.device_list() {
        let e = by.entry((d.vendor_id(), d.product_id())).or_insert_with(|| {
            (d.manufacturer_string().unwrap_or("").to_string(), d.product_string().unwrap_or("").to_string(), 0, 0)
        });
        e.2 += 1;
        if d.usage_page() >= 0xFF00 {
            e.3 += 1;
        }
    }
    println!("# HID devices (VID:PID  manufacturer / product  interfaces, vendor-defined)\n");
    for ((vid, pid), (man, prod, n, vendor)) in by {
        println!("{vid:04X}:{pid:04X}  {man} / {prod}  {n} interfaces, {vendor} vendor-defined");
    }
    println!("\nNext: periph-pack probe --vid <VID> [--pid <PID>] for the details of one device.");
}

fn matching<'a>(api: &'a HidApi, vid: u16, pid: Option<u16>) -> Vec<&'a DeviceInfo> {
    let mut v: Vec<&DeviceInfo> = api.device_list().filter(|d| d.vendor_id() == vid && pid.is_none_or(|p| d.product_id() == p)).collect();
    v.sort_by_key(|d| (d.product_id(), d.interface_number(), d.usage_page(), d.usage()));
    v
}

/// Interfaces of one device with their report descriptors.
pub fn describe(api: &HidApi, vid: u16, pid: Option<u16>) {
    let devs = matching(api, vid, pid);
    if devs.is_empty() {
        println!("No HID device {vid:04X}{} found.", pid.map(|p| format!(":{p:04X}")).unwrap_or_default());
        return;
    }
    println!("# HID interfaces of {vid:04X}{}\n", pid.map(|p| format!(":{p:04X}")).unwrap_or_default());
    for d in devs {
        println!(
            "## PID {:04X}  MI_{:02}  usagePage 0x{:04X}  usage 0x{:04X}{}",
            d.product_id(),
            d.interface_number(),
            d.usage_page(),
            d.usage(),
            if d.usage_page() >= 0xFF00 { "  [vendor-defined: configuration usually happens here]" } else { "" }
        );
        println!("  product={:?} manufacturer={:?}", d.product_string().unwrap_or(""), d.manufacturer_string().unwrap_or(""));
        match d.open_device(api) {
            Err(e) => println!("  (could not open: {e}; normal for mouse/keyboard collections owned by Windows)\n"),
            Ok(dev) => {
                let mut buf = [0u8; 4096];
                match dev.get_report_descriptor(&mut buf) {
                    Ok(n) => print!("{}", descriptor::parse(&buf[..n]).to_text()),
                    Err(e) => println!("  (report descriptor unavailable: {e})"),
                }
                println!();
            }
        }
    }
}

/// Print every input report the vendor-defined interfaces send, for `secs` seconds.
pub fn listen(api: &HidApi, vid: u16, pid: Option<u16>, secs: u64) -> Result<()> {
    let opened: Vec<_> = matching(api, vid, pid)
        .into_iter()
        .filter(|d| d.usage_page() >= 0xFF00)
        .filter_map(|d| d.open_device(api).ok().map(|h| (d, h)))
        .collect();
    for (_, h) in &opened {
        h.set_blocking_mode(false)?;
    }
    println!("# Listening {secs}s on {} vendor-defined interfaces (use the device, wait for battery updates)\n", opened.len());
    let end = Instant::now() + Duration::from_secs(secs);
    let mut buf = [0u8; 1024];
    while Instant::now() < end {
        for (d, h) in &opened {
            if let Ok(n) = h.read(&mut buf) {
                if n > 0 {
                    println!("[PID {:04X} 0x{:04X}/0x{:04X}] {}", d.product_id(), d.usage_page(), d.usage(), hex(&buf[..n]));
                }
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
