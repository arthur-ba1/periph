//! The `reports` transport on a made-up device (tests/fixtures/sample-reports): reads,
//! templated writes with a checksum, "save" after writes, write-only settings.

use periph_pack::engine::{Cmd, Engine, State};
use periph_pack::port::SimReports;
use periph_pack::Package;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

fn pkg() -> Package {
    Package::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample-reports")).unwrap()
}

fn sim(pkg: &Package) -> SimReports {
    let answers = BTreeMap::from([("config".to_string(), vec![0x01, 0x90, 0x01, 0x03, 0x01, 0x10, 0x20])]);
    SimReports::new(pkg, &answers).unwrap()
}

fn set(id: &str, index: Option<usize>, value: Value) -> Cmd {
    Cmd::Set { id: id.into(), index, value }
}

#[test]
fn examples_pass() {
    assert_eq!(periph_pack::check(&pkg()), Vec::<String>::new());
}

#[test]
fn reads_the_answer() {
    let pkg = pkg();
    let mut s = sim(&pkg);
    let c = Engine::new(&pkg, &mut s, &mut State::default()).read_config().unwrap();
    assert_eq!(c, json!({ "dpi": 400, "led": true, "mode": "fast", "colors": [16, 32], "sleep": 10, "firmware": 1 }));
    // the read request itself carries the checksum
    assert_eq!(s.requests[0], [0x04, 0x01, 0, 0, 0, 0, 0, 0x01]);
}

#[test]
fn write_only_settings_are_remembered() {
    let pkg = pkg();
    let mut s = sim(&pkg);
    let mut state = State::default();
    Engine::new(&pkg, &mut s, &mut state).read_config().unwrap();
    Engine::new(&pkg, &mut s, &mut state).apply(set("sleep", None, json!(20))).unwrap();
    let c = Engine::new(&pkg, &mut s, &mut state).read_config().unwrap();
    assert_eq!(c["sleep"], json!(20));
}

#[test]
fn invalid_writes_send_nothing() {
    let pkg = pkg();
    for cmd in [
        set("sleep", None, json!(40)),
        set("dpi", None, json!(825)),
        set("mode", None, json!("medium")),
        set("firmware", None, json!(2)),
        set("colors", Some(2), json!(1)),
        set("colors", None, json!(1)),
        set("nope", None, json!(1)),
    ] {
        let mut s = sim(&pkg);
        let mut state = State::default();
        Engine::new(&pkg, &mut s, &mut state).read_config().unwrap();
        let shown = format!("{cmd:?}");
        assert!(Engine::new(&pkg, &mut s, &mut state).apply(cmd).is_err(), "{shown}");
        assert!(s.writes().is_empty(), "{shown}");
    }
}
