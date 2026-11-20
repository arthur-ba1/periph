//! Smoke test for the virtual DualSense kernel driver, without the Periph UI.
//!
//!   cargo run -p dualsense-vpad --example smoke -- [--seconds 30] [--forward-feedback] [--keep]
//!
//! 1. finds the physical pad (close DSX or any other emulator first) and reports whether HidHide
//!    is hiding it (read-only GET requests);
//! 2. reads its feature reports (GET_FEATURE only), uploads them and starts the virtual pad;
//! 3. forwards input for N seconds and prints what games write back (rumble, lightbar, ...).
//!    Output reports are only written to the physical pad with `--forward-feedback`;
//! 4. destroys the virtual pad, or with `--keep` just exits so the kernel's close-handle cleanup
//!    is what removes it (check that `joy.cpl` no longer lists it).

#[cfg(not(windows))]
fn main() {
    eprintln!("Windows only");
}

#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    use anyhow::{bail, Context};
    use dualsense_proto::feature::{FeatureResponse, ALLOWED};
    use dualsense_proto::input::InputReport;
    use dualsense_proto::{PID, VID};
    use dualsense_vpad::windows_channel::WindowsChannel;
    use dualsense_vpad::Channel;
    use hidapi::HidApi;
    use std::time::{Duration, Instant};

    let args: Vec<String> = std::env::args().collect();
    let seconds: u64 = args
        .iter()
        .position(|a| a == "--seconds")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(30);
    let forward = args.iter().any(|a| a == "--forward-feedback");
    let keep = args.iter().any(|a| a == "--keep");

    // 1. Physical pad.
    let api = HidApi::new()?;
    let pads: Vec<_> = api
        .device_list()
        .filter(|d| d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == 0x01 && d.usage() == 0x05)
        .collect();
    println!("DualSense HID collections: {}", pads.len());
    for d in &pads {
        println!("  {}", d.path().to_string_lossy());
    }
    if pads.len() != 1 {
        bail!("expected exactly one DualSense (close DSX / other emulators, connect the pad by USB)");
    }
    let info = pads[0];
    let path = info.path().to_string_lossy().into_owned();

    if let Some(mut ctl) = hidhide::open()? {
        use hidhide::Control;
        let black = ctl.blacklist()?;
        let pad_entries: Vec<_> = black.iter().filter(|e| e.to_ascii_uppercase().contains("VID_054C&PID_0CE6")).collect();
        println!(
            "HidHide: active={} inverse={} DualSense entries in blacklist: {:?}",
            ctl.active()?,
            ctl.inverse()?,
            pad_entries
        );
    } else {
        println!("HidHide: not installed");
    }

    let pad = info.open_device(&api).context("open the physical pad (hidden by HidHide?)")?;

    // 2. Features + virtual pad.
    let mut ch = WindowsChannel::open()?;
    println!("channel: opened \\\\.\\PeriphVpad, active before start = {}", ch.is_active()?);
    for (id, len) in ALLOWED {
        let mut buf = vec![0u8; *len];
        buf[0] = *id;
        match pad.get_feature_report(&mut buf) {
            Ok(n) => match FeatureResponse::new(*id, &buf[..n]) {
                Ok(resp) => {
                    ch.set_feature_response(&resp)?;
                    println!("feature 0x{id:02X}: {n} bytes uploaded");
                }
                Err(e) => println!("feature 0x{id:02X}: rejected ({e})"),
            },
            Err(e) => println!("feature 0x{id:02X}: read failed ({e})"),
        }
    }
    ch.start()?;
    std::thread::sleep(Duration::from_millis(1500));
    println!("virtual pad active = {}", ch.is_active()?);
    let api2 = HidApi::new()?;
    for d in api2.device_list().filter(|d| d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == 0x01) {
        let p = d.path().to_string_lossy();
        if p != path {
            println!("  new HID device: {p}");
        }
    }

    // 3. Forward input, watch output.
    println!("forwarding for {seconds} s (move the sticks; open joy.cpl or a game). forward-feedback={forward}");
    let mut buf = [0u8; dualsense_proto::INPUT_REPORT_LEN];
    let (mut inputs, mut outputs) = (0u64, 0u64);
    let start = Instant::now();
    let mut last_print = Instant::now();
    while start.elapsed() < Duration::from_secs(seconds) {
        let n = pad.read_timeout(&mut buf, 10)?;
        if n > 0 {
            if let Ok(report) = InputReport::parse(&buf[..n]) {
                ch.submit_input(&report)?;
                inputs += 1;
            }
        }
        while let Some(out) = ch.read_output()? {
            outputs += 1;
            println!(
                "  output #{outputs}: flags={:02X} {:02X} rumble={:?} lightbar={:?} player={:02X} mic={} R2mode={:02X} L2mode={:02X}",
                out.raw[1],
                out.raw[2],
                out.rumble(),
                out.lightbar_rgb(),
                out.player_leds(),
                out.mic_led(),
                out.right_trigger()[0],
                out.left_trigger()[0]
            );
            if forward {
                pad.write(out.as_bytes())?;
            }
        }
        if last_print.elapsed() >= Duration::from_secs(1) {
            println!("  {}s: inputs={inputs} outputs={outputs}", start.elapsed().as_secs());
            last_print = Instant::now();
        }
    }

    // 4. Teardown.
    if forward {
        let _ = pad.write(dualsense_proto::output::OutputReport::rumble_stop().as_bytes());
    }
    if keep {
        println!("exiting without DESTROY: the kernel should remove the pad when the handle closes");
    } else {
        ch.destroy()?;
        std::thread::sleep(Duration::from_millis(500));
        println!("destroyed; active = {}", ch.is_active()?);
    }
    Ok(())
}
