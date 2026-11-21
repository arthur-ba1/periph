//! Device packages: support for a peripheral described in JSON and loaded at runtime, so
//! nobody has to rebuild Periph to add their own device. Format: `docs/device-packages.md`.
//!
//! - [`spec`]: the files as written by the author
//! - [`Package`]: a loaded, validated package folder
//! - [`engine`]: reads the config and applies commands over a [`port::Port`]
//! - [`PackageDriver`]: a package as a `periph_core::Driver`
//! - [`check`]: replays the package's captured examples against a simulated device

pub mod checksum;
pub mod descriptor;
pub mod driver;
pub mod engine;
pub mod package;
pub mod port;
pub mod probe;
pub mod scaffold;
pub mod spec;
pub mod template;
pub mod value;

pub use driver::PackageDriver;
pub use package::{discover, Package};

use engine::{Cmd, Engine, State};
use port::{Port, SimRegisters, SimReports};
use spec::{parse_hex, Example, Transport};
use std::collections::BTreeMap;

fn hex(v: &[u8]) -> String {
    v.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

/// Run one example; `Err` says why it failed.
fn run_example(pkg: &Package, ex: &Example) -> Result<(), String> {
    let mut set = ex.set.clone();
    if let Some(o) = set.as_object_mut() {
        o.insert("type".into(), "set".into());
    }
    let cmd: Cmd = serde_json::from_value(set).map_err(|e| format!("bad \"set\": {e}"))?;
    let want: Vec<Vec<u8>> = ex.writes.iter().map(|w| parse_hex(w)).collect::<Result<_, _>>()?;
    let mut state = State::default();
    let got: Vec<Vec<u8>> = match &pkg.device.transport {
        Transport::Registers(r) => {
            let mut sim = SimRegisters::new(r);
            for (a, b) in ex.memory()? {
                sim.load(a, &b);
            }
            Engine::new(pkg, &mut sim, &mut state).apply(cmd).map_err(|e| format!("rejected: {e}"))?;
            sim.writes().into_iter().cloned().collect()
        }
        Transport::Reports(_) => {
            let answers: BTreeMap<String, Vec<u8>> = ex.responses.iter().map(|(k, v)| Ok((k.clone(), parse_hex(v)?))).collect::<Result<_, String>>()?;
            let mut sim = SimReports::new(pkg, &answers).map_err(|e| e.to_string())?;
            // Like a session: the state is read first, then the command applied.
            if !pkg.device.reads.is_empty() {
                Engine::new(pkg, &mut sim as &mut dyn Port, &mut state).read_config().map_err(|e| format!("reading the responses: {e}"))?;
            }
            Engine::new(pkg, &mut sim, &mut state).apply(cmd).map_err(|e| format!("rejected: {e}"))?;
            sim.writes().into_iter().cloned().collect()
        }
    };
    let same = got.len() == want.len()
        && got.iter().zip(&want).all(|(g, w)| g.len() >= w.len() && g[..w.len()] == w[..] && g[w.len()..].iter().all(|b| *b == 0));
    if same {
        return Ok(());
    }
    let trim = |v: &Vec<u8>| hex(&v[..v.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1)]);
    Err(format!(
        "writes differ\n    expected: {}\n    got:      {}",
        want.iter().map(|w| hex(w)).collect::<Vec<_>>().join(" | "),
        got.iter().map(trim).collect::<Vec<_>>().join(" | ")
    ))
}

/// Replay every example of the package on a simulated device. Returns the failures.
pub fn check(pkg: &Package) -> Vec<String> {
    pkg.device
        .examples
        .iter()
        .enumerate()
        .filter_map(|(i, ex)| {
            let name = format!("example {} ({})", i + 1, ex.note.as_deref().unwrap_or(&ex.set.to_string()));
            run_example(pkg, ex).err().map(|e| format!("{name}: {e}"))
        })
        .collect()
}
