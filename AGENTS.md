# Periph: notes for AI agents

Periph is an open-source desktop app (Rust + Tauri 2 + Svelte 5) that replaces vendor
configuration software for computer peripherals. Each supported device is a pluggable
implementation: a Rust driver plus a Svelte module that share an id.

## Layout

- `crates/periph-core`: the driver contract (`Driver`, `Session`) and the generic worker loop
- `crates/periph-app`: the Tauri shell (binary `periph`); `src/drivers/` holds one file per
  device and the registry in `drivers/mod.rs`; `src/tray.rs` is the generic tray
- `crates/vt2-proto`, `crates/vt2-device`: Rapoo VT2 Max protocol (pure) and HID transport
- `crates/via`: VIA keyboard protocol; definitions live in `app/src/devices/via/keyboards/`
- `crates/vt2-probe`: read-only HID discovery (`vt2-probe --vid <hex>`, `via-probe`)
- `app/src/devices/<id>/`: device UI modules, auto-discovered; `app/src/lib/devices.svelte.js`
  is the registry, `app/src/lib/device-state.svelte.js` the shared state and `send()`
- `tools/`: reverse-engineering helpers (Frida HID hook, log summarizer, resource extractors)
- `docs/`: protocol notes, `adding-a-device.md` (contract), `reverse-engineering.md` (method)

## Adding or extending a device

Use the skill at `.claude/skills/add-device/SKILL.md` (Claude Code loads it automatically;
other agents: read it and follow it). It covers identification, capturing the vendor app,
decoding, the protocol crate, the driver, the UI module and verification.

## Rules

- Hardware safety first: only send commands observed from the official software; no fuzzing,
  no firmware/bootloader/factory-reset commands; discovery tools stay read-only.
- Vendor artwork is copyrighted: `app/src/devices/*/assets/` is git-ignored (except
  `NOTICE.md`); code must work when those files are missing.
- UI copy is Portuguese (pt-BR); code, comments and docs are English.

## Commands

```
cargo test --workspace
npm --prefix app install
npm --prefix app run build             # frontend into app/dist (embedded in the exe)
cargo build --release -p periph-app    # target/release/periph.exe
npm --prefix app run dev               # UI only, http://localhost:5173/?demo (fake devices)
```
