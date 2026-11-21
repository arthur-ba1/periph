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
- `crates/dualsense-proto`, `crates/dualsense-vpad`, `crates/hidhide`: DualSense reports (pure),
  user-mode channel to the virtual pad driver, HidHide client (hide the physical pad)
- The virtual DualSense kernel driver (`periph-vpad`, KMDF + VHF) lives in its own repository;
  `crates/dualsense-vpad` is its client. See `docs/dualsense-implementation.md`
- `crates/periph-pack`: device packages (JSON devices loaded at runtime, no rebuild) and the engine
  that runs them; example packages in `packages/`. See `docs/device-packages.md`
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
- No user-visible literal strings: UI, tray and user-facing errors go through `t()` with keys in
  every language file (`app/src/locales/`, `app/src/devices/<id>/locales/`). See
  `docs/translating.md`. `npm --prefix app run check:i18n` must pass.
- Code, comments and docs are English; the READMEs exist in English and Portuguese.
- Commits and PRs carry no attribution trailers: no `Co-Authored-By:` and no
  "Generated with Claude Code". The `.githooks/commit-msg` hook (enable with
  `git config core.hooksPath .githooks`) and CI reject them; `.claude/settings.json` turns
  Claude Code's attribution off. Never bypass the hook.

## Commands

```
git config core.hooksPath .githooks   # once per clone
cargo test --workspace
npm --prefix app install
npm --prefix app run check:i18n
npm --prefix app run build             # frontend into app/dist (embedded in the exe)
cargo build --release -p periph-app    # target/release/periph.exe
npm --prefix app run dev               # UI only, http://localhost:5173/?demo (fake devices)
```
