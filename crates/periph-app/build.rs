use std::path::{Path, PathBuf};

fn json_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect())
        .unwrap_or_default();
    v.sort();
    v
}

fn include(p: &Path) -> String {
    format!("include_str!({:?})", p.canonicalize().unwrap().to_string_lossy())
}

fn main() {
    let app = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Every VIA definition in app/src/devices/via/keyboards/ (adding a keyboard = adding a JSON).
    let kb = app.join("devices/via/keyboards");
    println!("cargo:rerun-if-changed={}", kb.display());
    let entries: String = json_files(&kb).iter().map(|p| format!("({:?}, {}),\n", p.file_stem().unwrap().to_string_lossy(), include(p))).collect();
    std::fs::write(out.join("via_definitions.rs"), format!("&[\n{entries}]")).unwrap();

    // Translation catalogs shared with the frontend: app/src/locales/<lang>.json (namespace "")
    // and app/src/devices/<id>/locales/<lang>.json (namespace "<id>").
    let mut locales = String::new();
    let mut add = |dir: PathBuf, ns: &str| {
        println!("cargo:rerun-if-changed={}", dir.display());
        for p in json_files(&dir) {
            locales += &format!("({:?}, {ns:?}, {}),\n", p.file_stem().unwrap().to_string_lossy(), include(&p));
        }
    };
    add(app.join("locales"), "");
    let devices = app.join("devices");
    println!("cargo:rerun-if-changed={}", devices.display());
    let mut ids: Vec<_> = std::fs::read_dir(&devices).map(|rd| rd.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    ids.sort();
    for d in ids {
        let id = d.file_name().unwrap().to_string_lossy().to_string();
        if !id.starts_with('_') {
            add(d.join("locales"), &id);
        }
    }
    std::fs::write(out.join("locales.rs"), format!("&[\n{locales}]")).unwrap();

    // Explicit rerun-if-changed lines replace Cargo's default "any file in the package", so the
    // inputs tauri_build embeds (exe icon resource, config) must be listed too.
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::build()
}
