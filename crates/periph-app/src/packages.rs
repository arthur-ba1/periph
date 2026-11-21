//! Device packages (`docs/device-packages.md`): JSON device descriptions loaded at startup from
//! `%APPDATA%\Periph\devices\` and the folders in `PERIPH_PACKAGES`, run by `periph-pack`.

use periph_core::Driver;
use periph_pack::{Package, PackageDriver};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Folders that hold package folders.
pub fn dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        v.push(PathBuf::from(appdata).join("Periph").join("devices"));
    }
    if let Some(extra) = std::env::var_os("PERIPH_PACKAGES") {
        v.extend(std::env::split_paths(&extra));
    }
    v
}

/// What the UI needs to render a package (it builds the pages from the settings).
fn ui(p: &Package) -> Value {
    json!({ "manifest": p.manifest_json, "device": p.device_json, "locales": p.locales })
}

/// Add the installed packages to the built-in drivers: a package that `replaces` a built-in
/// driver removes it, and a package whose id clashes with one is skipped. Returns
/// `{ packages: [UI payload], failed: [{ dir, error }] }` for the UI.
pub fn install(drivers: &mut Vec<Box<dyn Driver>>) -> Value {
    let mut out = Vec::new();
    let mut failed = Vec::new();
    for (dir, loaded) in periph_pack::discover(&dirs()) {
        let pkg = match loaded {
            Ok(p) => p,
            Err(e) => {
                eprintln!("package skipped: {e:#}");
                failed.push(json!({ "dir": dir.display().to_string(), "error": format!("{e:#}") }));
                continue;
            }
        };
        let id = pkg.manifest.id.clone();
        if let Some(r) = &pkg.manifest.replaces {
            drivers.retain(|d| d.info().id != r);
        }
        if drivers.iter().any(|d| d.info().id == id) {
            let error = format!("a driver with the id \"{id}\" already exists");
            eprintln!("package {id} skipped: {error}");
            failed.push(json!({ "dir": dir.display().to_string(), "error": error }));
            continue;
        }
        for (lang, catalog) in &pkg.locales {
            if let Err(e) = periph_core::i18n::load(lang, &id, &catalog.to_string()) {
                eprintln!("package {id} locale {lang}: {e}");
            }
        }
        out.push(ui(&pkg));
        drivers.push(Box::new(PackageDriver::new(pkg)));
    }
    json!({ "packages": out, "failed": failed })
}
