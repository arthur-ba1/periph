//! The example package `packages/rapoo-vt2-max` against the built-in VT2 implementation:
//! every write it produces must be byte-for-byte what `vt2-proto` (used by the `vt2` driver)
//! produces for the same change, and it must decode the same values.

use periph_core::Snapshot;
use periph_pack::engine::{parse_status, Cmd, Engine, State};
use periph_pack::port::SimRegisters;
use periph_pack::spec::Transport;
use periph_pack::Package;
use serde_json::{json, Value};
use std::path::Path;
use vt2_proto::{self as p, addr, ButtonAction};

fn pkg() -> Package {
    Package::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/rapoo-vt2-max")).unwrap()
}

/// A register map with plausible contents (bytes that a write must carry over unchanged).
fn sim(pkg: &Package) -> SimRegisters {
    let Transport::Registers(r) = &pkg.device.transport else { unreachable!() };
    let mut s = SimRegisters::new(r);
    for a in 0x0600..0x0900u32 {
        s.load(a, &[(a as u8).wrapping_mul(37) ^ 0x5A]);
    }
    s.load(addr::POLLING as u32, &[0x01, 0x06, 0xFF, 0xFF]);
    s.load(addr::LOD as u32, &[5, 1]);
    s.load(addr::DEBOUNCE_PRESS as u32, &[4, 4, 0, 0b10, 0xFE, 0]);
    let dpis: Vec<u8> = [400u16, 800, 1600, 3200, 6400, 12800, 26000].iter().flat_map(|d| d.to_le_bytes()).collect();
    s.load(addr::DPI_X as u32, &dpis);
    s.load(addr::DPI_Y as u32, &dpis);
    s.load(addr::DPI_LAST_INDEX as u32, &[4, 0, 1, 0, 1, 0]);
    for i in 0..addr::BUTTON_COUNT {
        s.load(p::button_addr(i) as u32, &ButtonAction::Mouse(1 << (i % 5)).encode());
    }
    s
}

fn set(id: &str, index: Option<usize>, value: Value) -> Cmd {
    Cmd::Set { id: id.into(), index, value }
}

/// Apply on a fresh simulated device; the write requests, unpadded to the proto's length.
fn writes(cmd: Cmd) -> Vec<Vec<u8>> {
    let pkg = pkg();
    let mut s = sim(&pkg);
    Engine::new(&pkg, &mut s, &mut State::default()).apply(cmd).unwrap();
    s.writes().into_iter().cloned().collect()
}

fn padded(v: Vec<u8>) -> Vec<u8> {
    p::pad(v)
}

/// What `Vt2::set_byte` writes: its 4-byte block with the first byte replaced.
fn set_byte(mem: &SimRegisters, at: u16, value: u8) -> Vec<u8> {
    let mut block: Vec<u8> = (0..4).map(|i| mem.memory[&(at as u32 + i)]).collect();
    block[0] = value;
    padded(p::write_cmd(at, &block).unwrap())
}

#[test]
fn examples_pass() {
    let pkg = pkg();
    assert_eq!(periph_pack::check(&pkg), Vec::<String>::new());
    assert_eq!(pkg.pending_code, Some("asleep"));
}

#[test]
fn single_byte_settings_match_vt2() {
    let pkg = pkg();
    let mem = sim(&pkg);
    for hz in [125, 250, 500, 1000, 2000, 4000, 8000] {
        assert_eq!(writes(set("polling", None, json!(hz))), [set_byte(&mem, addr::POLLING, p::polling_code(hz).unwrap())], "{hz}");
    }
    for mm in [0.7f32, 1.2, 1.7] {
        assert_eq!(writes(set("lod", None, json!(mm))), [set_byte(&mem, addr::LOD, p::lod_raw(mm).unwrap())], "{mm}");
    }
    for on in [false, true] {
        assert_eq!(writes(set("motionSync", None, json!(on))), [set_byte(&mem, addr::MOTION_SYNC, on as u8)]);
        assert_eq!(writes(set("glass", None, json!(on))), [set_byte(&mem, addr::GLASS, on as u8)]);
    }
    assert_eq!(writes(set("debounce", None, json!(12))), [set_byte(&mem, addr::DEBOUNCE_PRESS, 12)]);
}

#[test]
fn correction_bits_match_vt2() {
    // vt2 driver: on clears the bit, off sets it (linear = bit 0, ripple = bit 1).
    let pkg = pkg();
    let mem = sim(&pkg);
    let cur = mem.memory[&(addr::CORRECTION_FLAGS as u32)];
    for (id, bit) in [("linear", 1u8), ("ripple", 2)] {
        for on in [false, true] {
            let raw = if on { cur & !bit } else { cur | bit };
            assert_eq!(writes(set(id, None, json!(on))), [set_byte(&mem, addr::CORRECTION_FLAGS, raw)], "{id} {on}");
        }
    }
}

#[test]
fn stages_match_vt2() {
    let pkg = pkg();
    let mem = sim(&pkg);
    // Vt2::set_stage_dpi: whole X block (0x14) then whole Y block (0x10), both patched.
    let block = |at: u16, len: u32| -> Vec<u8> { (0..len).map(|i| mem.memory[&(at as u32 + i)]).collect() };
    let (mut x, mut y) = (block(addr::DPI_X, 0x14), block(addr::DPI_Y, 0x10));
    x[6..8].copy_from_slice(&1230u16.to_le_bytes());
    y[6..8].copy_from_slice(&1230u16.to_le_bytes());
    assert_eq!(
        writes(set("dpi", Some(3), json!(1230))),
        [padded(p::write_cmd(addr::DPI_X, &x).unwrap()), padded(p::write_cmd(addr::DPI_Y, &y).unwrap())]
    );
    // Vt2::set_active_stage: bytes 0 and 2 of the 4-byte block.
    let mut a = block(addr::DPI_ACTIVE, 4);
    a[0] = 3;
    a[2] = 3;
    assert_eq!(writes(set("active", None, json!(3))), [padded(p::write_cmd(addr::DPI_ACTIVE, &a).unwrap())]);
}

#[test]
fn buttons_match_vt2() {
    for (v, action) in [
        (json!({ "kind": "mouse", "value": 2 }), ButtonAction::Mouse(2)),
        (json!({ "kind": "key", "value": 0x1E }), ButtonAction::Key(0x1E)),
        (json!({ "kind": "dpiUp" }), ButtonAction::DpiUp),
        (json!({ "kind": "dpiDown" }), ButtonAction::DpiDown),
        (json!({ "kind": "disabled" }), ButtonAction::Disabled),
    ] {
        assert_eq!(writes(set("buttons", Some(9), v)), [padded(p::write_cmd(p::button_addr(9), &action.encode()).unwrap())]);
    }
}

#[test]
fn unsafe_values_are_rejected_without_writing() {
    let pkg = pkg();
    for cmd in [
        set("dpi", Some(0), json!(805)),        // not a multiple of 10
        set("dpi", Some(0), json!(30000)),      // above the range
        set("dpi", Some(7), json!(800)),        // no 8th stage
        set("dpi", None, json!(800)),           // array needs an index
        set("polling", None, json!(300)),       // not in the table
        set("lod", None, json!(2.0)),
        set("active", None, json!(7)),
        set("angle", None, json!(0)),           // read-only
        set("debounceRelease", None, json!(4)), // read-only
        set("nope", None, json!(1)),
        set("buttons", Some(0), json!({ "kind": "wheel", "value": 1 })), // decoded, never observed written
        set("buttons", Some(0), json!({ "kind": "raw", "bytes": "01 02 03 04" })),
        set("buttons", Some(0), json!({ "kind": "key", "value": 200 })),
        set("buttons", Some(0), json!({ "kind": "mouse", "value": 3 })),
        set("buttons", Some(0), json!({ "kind": "dpiUp", "value": 1 })),
    ] {
        let mut s = sim(&pkg);
        let shown = format!("{cmd:?}");
        assert!(Engine::new(&pkg, &mut s, &mut State::default()).apply(cmd).is_err(), "{shown}");
        assert!(s.writes().is_empty(), "{shown}");
    }
}

#[test]
fn config_decodes_like_vt2() {
    let pkg = pkg();
    let mut s = sim(&pkg);
    let c = Engine::new(&pkg, &mut s, &mut State::default()).read_config().unwrap();
    assert_eq!(c["dpi"], json!([400, 800, 1600, 3200, 6400, 12800, 26000]));
    assert_eq!(c["stageCount"], json!(5));
    assert_eq!(c["active"], json!(1));
    assert_eq!(c["polling"], json!(1000));
    assert_eq!(c["lod"], json!(1.1));
    assert_eq!(c["motionSync"], json!(true));
    assert_eq!(c["linear"], json!(true)); // flags 0b10: bit 0 clear = on
    assert_eq!(c["ripple"], json!(false));
    assert_eq!(c["glass"], json!(false));
    assert_eq!(c["angle"], json!(-2));
    assert_eq!(c["debounce"], json!(4));
    assert_eq!(c["buttons"][0], json!({ "kind": "mouse", "value": 1 }));
    assert_eq!(c["buttons"][3], json!({ "kind": "mouse", "value": 8 }));
}

#[test]
fn status_matches_vt2() {
    let pkg = pkg();
    let hex = |s: &str| -> Vec<u8> { s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect() };
    for report in ["07 20 01 2A 03 2A 03 01 53 00 84 84 00 00 00 00 06 04 00", "07 22 02 B0 04 B0 04 02 55 00 01 01 00 00 00 00 20 00 00"] {
        let b = hex(report);
        let want = p::parse_status(&b).unwrap();
        let got = parse_status(&pkg, &b).unwrap();
        assert_eq!(got["stage"], json!(want.dpi_stage));
        assert_eq!(got["dpi"], json!(want.dpi_x));
        assert_eq!(got["battery"], json!(want.battery));
        assert_eq!(got["charging"], json!(want.charging));
        assert_eq!(got["wired"], json!(want.wired));
        assert_eq!(got["polling"], json!(want.polling_hz.unwrap()));
    }
    assert!(parse_status(&pkg, &hex("06 01 00 00")).is_none());
}

#[test]
fn tray_items_send_valid_commands() {
    let pkg = pkg();
    let mut s = sim(&pkg);
    let config = Engine::new(&pkg, &mut s, &mut State::default()).read_config().unwrap();
    let snap = Snapshot { connected: true, config: Some(config), status: Some(json!({ "battery": 80, "charging": true, "dpi": 800 })), ..Default::default() };
    let tray = periph_pack::driver::tray(&pkg, &snap).unwrap();
    assert_eq!(tray.battery, Some((80, true)));
    assert_eq!(tray.items.len(), 5);
    for id in ["refresh", "set:motionSync", "set:glass", "set:active:2", "set:polling:6"] {
        let cmd = periph_pack::driver::tray_command(&pkg, id, &snap).unwrap_or_else(|| panic!("{id}"));
        let cmd: Cmd = serde_json::from_value(cmd).unwrap();
        let mut s = sim(&pkg);
        Engine::new(&pkg, &mut s, &mut State::default()).apply(cmd).unwrap_or_else(|e| panic!("{id}: {e}"));
    }
}
