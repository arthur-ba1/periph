//! Hardware check of `packages/rapoo-vt2-max` against the built-in VT2 transport, on a real
//! mouse. Close Periph first (both would use the same HID channel).
//!
//!   cargo run -p periph-pack --example vt2_hardware            read-only comparison
//!   cargo run -p periph-pack --example vt2_hardware -- --write also change the polling rate
//!                                                              and put it back

use periph_core::Driver;
use periph_pack::{Package, PackageDriver};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use vt2_device::Vt2;
use vt2_proto::{self as p, addr};

fn builtin() -> anyhow::Result<Value> {
    let d = Vt2::open()?;
    let (xs, _, last, active) = d.dpi_stages()?;
    let flags = d.get_byte(addr::CORRECTION_FLAGS)?;
    let sensor = d.read(addr::DEBOUNCE_PRESS, 6)?;
    let lm = d.read(addr::LOD, 2)?;
    Ok(json!({
        "dpi": xs, "stageCount": last as u64 + 1, "active": active, "polling": d.polling()?,
        "lod": (p::lod_mm(lm[0]) as f64 * 10.0).round() / 10.0, "motionSync": lm[1] != 0,
        "linear": flags & 1 == 0, "ripple": flags & 2 == 0, "glass": sensor[5] != 0,
        "angle": sensor[4] as i8, "debounce": sensor[0], "debounceRelease": sensor[1],
    }))
}

fn main() -> anyhow::Result<()> {
    let write = std::env::args().any(|a| a == "--write");
    let want = builtin()?;
    println!("built-in: {want}");

    let pkg = Package::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/rapoo-vt2-max"))?;
    let driver = PackageDriver::new(pkg);
    let (mut s, info) = driver.open()?;
    println!("package opened: {} {}", info.product, info.serial);
    let got = s.read_config()?;
    println!("package:  {got}");
    let mut diffs = 0;
    for (k, v) in want.as_object().unwrap() {
        let g = &got[k];
        let same = match (v.as_f64(), g.as_f64()) {
            (Some(a), Some(b)) => (a - b).abs() < 1e-6,
            _ => v == g,
        };
        if !same {
            diffs += 1;
            println!("  DIFF {k}: built-in {v} / package {g}");
        }
    }
    println!("buttons:  {}", got["buttons"]);
    println!("config: {}", if diffs == 0 { "same values" } else { "DIFFERENT" });

    for _ in 0..4 {
        if let Some(st) = s.poll()? {
            println!("status:   {st}");
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    if write {
        let orig = got["polling"].as_u64().unwrap();
        let other = if orig == 500 { 1000 } else { 500 };
        for hz in [other, orig] {
            s.apply(json!({ "type": "set", "id": "polling", "value": hz }))?;
            let now = s.read_config()?["polling"].clone();
            let check = Vt2::open()?.polling()?;
            println!("set polling {hz}: package reads {now}, built-in reads {check}");
            anyhow::ensure!(now == json!(hz) && check as u64 == hz, "polling did not change to {hz}");
        }
        println!("write round trip ok, polling back to {orig} Hz");
    }
    Ok(())
}
