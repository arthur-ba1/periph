//! Debug: one read request on the VT2 command interface, raw answers printed (read-only).
fn main() -> anyhow::Result<()> {
    let api = hidapi::HidApi::new()?;
    let info = api.device_list().find(|d| d.vendor_id() == 0x24AE && d.usage_page() == 0xFF00 && d.usage() == 0x0E).unwrap();
    let dev = info.open_device(&api)?;
    let mut req = vec![0x06, 0xA5, 0xA4, 0x04, 0x80, 0x08, 0, 0];
    req.resize(33, 0);
    dev.write(&req)?;
    for _ in 0..5 {
        let mut buf = [0u8; 33];
        buf[0] = 6;
        let n = dev.get_input_report(&mut buf)?;
        println!("n={n} {:02X?}", &buf[..12]);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(())
}
