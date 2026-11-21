//! periph-pack: the toolbox for writing a device package (docs/device-packages.md).
//!
//!   periph-pack probe                                  list HID devices (read-only)
//!   periph-pack probe --vid 24AE [--pid 1423]          interfaces and report layouts
//!   periph-pack probe --vid 24AE --listen 10           print what the device sends on its own
//!   periph-pack new <folder> --vid .. --pid .. --usage-page .. --usage .. [--name ..]
//!                   [--kind mouse] [--transport registers|reports]
//!   periph-pack check <package folder>...              validate and replay the examples
//!   periph-pack dir                                    where Periph loads packages from
//!
//! Nothing here writes to a device.

use periph_pack::scaffold::{self, New};
use std::path::PathBuf;
use std::process::ExitCode;

fn hex_arg(args: &[String], name: &str) -> Result<Option<u16>, String> {
    match args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)) {
        None => Ok(None),
        Some(v) => u16::from_str_radix(v.trim_start_matches("0x").trim_start_matches("0X"), 16).map(Some).map_err(|_| format!("{name}: \"{v}\" is not a hex number")),
    }
}

fn arg<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

/// Where Periph looks for user packages (keep in sync with crates/periph-app/src/packages.rs).
fn user_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Periph").join("devices"))
}

fn check(dirs: &[String]) -> bool {
    let mut ok = true;
    for dir in dirs {
        match periph_pack::Package::load(&PathBuf::from(dir)) {
            Err(e) => {
                ok = false;
                eprintln!("FAIL {dir}\n  {e:#}");
            }
            Ok(pkg) => {
                let fails = periph_pack::check(&pkg);
                let n = pkg.device.examples.len();
                if fails.is_empty() {
                    println!("ok   {} {} ({} settings, {n} examples)", pkg.manifest.id, pkg.manifest.version, pkg.device.settings.len());
                    let writable = pkg.device.settings.iter().any(|s| !s.read_only && (s.write.is_some() || !s.send.is_empty() || !s.at.is_empty()));
                    if n == 0 && writable {
                        println!("     warning: no examples; its writes are not backed by captures");
                    }
                } else {
                    ok = false;
                    eprintln!("FAIL {}\n  {}", pkg.manifest.id, fails.join("\n  "));
                }
            }
        }
    }
    ok
}

fn run(args: &[String]) -> Result<bool, String> {
    let err = |e: anyhow::Error| e.to_string();
    match args.first().map(String::as_str) {
        Some("check") if args.len() > 1 => Ok(check(&args[1..])),
        Some("probe") => {
            let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
            match hex_arg(args, "--vid")? {
                None => periph_pack::probe::list(&api),
                Some(vid) => {
                    let pid = hex_arg(args, "--pid")?;
                    match arg(args, "--listen") {
                        Some(s) => periph_pack::probe::listen(&api, vid, pid, s.parse().map_err(|_| "--listen takes seconds")?).map_err(err)?,
                        None => periph_pack::probe::describe(&api, vid, pid),
                    }
                }
            }
            Ok(true)
        }
        Some("new") if args.len() > 1 => {
            let dir = PathBuf::from(&args[1]);
            let id = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let need = |n: &str| hex_arg(args, n).and_then(|v| v.ok_or(format!("{n} is required")));
            let n = New {
                id: &id,
                name: arg(args, "--name").unwrap_or(&id),
                kind: arg(args, "--kind").unwrap_or("other"),
                vid: need("--vid")?,
                pid: need("--pid")?,
                usage_page: need("--usage-page")?,
                usage: need("--usage")?,
                transport: arg(args, "--transport").unwrap_or("reports"),
            };
            scaffold::create(&dir, &n).map_err(err)?;
            println!("created {} (fill in device.json, then: periph-pack check {})", dir.display(), dir.display());
            Ok(true)
        }
        Some("dir") => {
            match user_dir() {
                Some(d) => println!("{}", d.display()),
                None => println!("APPDATA is not set"),
            }
            Ok(true)
        }
        _ => {
            eprintln!(
                "usage:\n  periph-pack probe [--vid <hex> [--pid <hex>] [--listen <s>]]\n  periph-pack new <folder> --vid <hex> --pid <hex> --usage-page <hex> --usage <hex> [--name <name>] [--kind mouse|keyboard|headset|controller|other] [--transport registers|reports]\n  periph-pack check <package folder>...\n  periph-pack dir"
            );
            Ok(false)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}
