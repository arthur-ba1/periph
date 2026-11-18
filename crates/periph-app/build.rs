use std::path::Path;

fn main() {
    // Embed every VIA definition in app/src/devices/via/keyboards/ (adding a keyboard = adding a JSON).
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src/devices/via/keyboards");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect())
        .unwrap_or_default();
    files.sort();
    let entries: String = files
        .iter()
        .map(|p| {
            let stem = p.file_stem().unwrap().to_string_lossy();
            let abs = p.canonicalize().unwrap();
            format!("({stem:?}, include_str!({:?})),\n", abs.to_string_lossy())
        })
        .collect();
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("via_definitions.rs");
    std::fs::write(out, format!("&[\n{entries}]")).unwrap();

    tauri_build::build()
}
