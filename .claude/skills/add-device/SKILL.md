---
name: add-device
description: Add support for a new peripheral (mouse, keyboard, headset, controller...) to Periph, from identifying the device and reverse engineering its vendor software's HID protocol to a working Rust driver and Svelte UI module. Use when someone wants Periph to configure a device it does not support yet, to add a VIA keyboard definition, or to extend an existing device with settings that are not decoded yet.
---

# Add a device to Periph

You are adding a device implementation: a Rust driver (`crates/periph-app/src/drivers/<id>.rs`,
usually backed by a pure protocol crate) and a Svelte module (`app/src/devices/<id>/`). The
contract and conventions are in `docs/adding-a-device.md`; the reverse-engineering method is in
`docs/reverse-engineering.md`. Read both before starting. `drivers/vt2.rs` + `devices/vt2/` is
the complete reference implementation; `templates/` (next to this file) is the minimal skeleton.

The user owns the hardware and the vendor app. Many steps need them to plug something in,
open the vendor app or click through a setting while you capture. Ask for exactly one action
at a time and say what you will do with it. If you have computer-use tools, you may drive the
vendor app yourself, still one change per capture marker.

## Safety (read first, applies to every step)

- Never send a byte sequence to the device that you have not seen the official software send.
  No guessing, no fuzzing, no "let's try opcode 0xA6".
- Never implement firmware update, bootloader, factory reset, pairing or calibration commands.
- Discovery tools only read. Before the driver's first write, the read path must already match
  what the vendor app shows.
- Writes accept only values the official app offers (allowlist in the protocol crate, tested).
- If something about the device is ambiguous, stop and ask the user rather than experiment on
  their hardware.
- Do not commit vendor artwork, binaries or raw captures containing serial numbers.

## Step 0: Gather facts

Ask the user (or find out) and write down:
- brand and exact model; wired / wireless dongle / Bluetooth
- the official configuration app and its version; whether it is installed on this machine
- which settings they care about (DPI, polling, buttons, lighting, ...)

Then choose the path:
- **QMK/VIA keyboard** (works with usevia.app): no driver. Get its VIA definition JSON (from
  the vendor or the `the-via/keyboards` repo), check `vendorId`/`productId` against
  `cargo run -p vt2-probe -- --vid <vid>`, drop it into `app/src/devices/via/keyboards/`, run
  `cargo test -p periph-app` and the demo. Done.
- **A device another open project already documents** (OpenRazer, libratbag/Piper, rivalcfg,
  HeadsetControl, ...): read their protocol notes and use them as a starting hypothesis, but
  still confirm every write against a capture of the official app.
- **Otherwise**: full process below.

Pick the id now: lowercase, short, stable (`g502x`, `viper3pro`). It names the driver file,
the module folder, the protocol crate (`<id>-proto`) and the `send(id, ...)` channel.

## Step 1: Identify HID interfaces (read-only)

Device plugged in, vendor app closed:
```
cargo run -p vt2-probe -- --vid <vid hex>
cargo run -p vt2-probe -- --vid <vid hex> --listen 10
```
Record PIDs (wired and dongle differ), vendor usage-page collections, report IDs, sizes and
types. Create `docs/<id>-protocol.md` now and keep filling it in.

## Step 2: Capture the official app

Follow `docs/reverse-engineering.md` sections 2 and 3: find whether a vendor DLL exports
hidapi (`--module`), start `tools/hidhook.py` writing to `captures/<id>-session.jsonl`, and
ask the user to perform the recipe one change at a time while you append markers to
`captures/marker.txt` (or they do). Always capture: app startup, idle, each setting at 2–3
values, each enumeration value, a couple of button remaps, unplug/replug, sleep/wake.

## Step 3: Decode

Use `python tools/summarize.py <log> --since-marker "<text>"`. Decode framing, the
busy/ready handshake, the memory map from the startup reads, then each setting and the status
reports (section 4 of the method doc). Every fact in `docs/<id>-protocol.md` cites its marker.
Mark anything unexplained as unknown; it stays read-only.

Show the user the decoded table and confirm values against what the vendor app displayed
before writing any code that writes.

## Step 4: Protocol crate

Copy `templates/proto/` to `crates/<id>-proto/` (rename `mydevice`), add it to the workspace
`Cargo.toml` members. Replace the example framing with the decoded one. Tests:
- every write builder reproduces a captured command byte for byte (paste the hex from the log)
- every parser decodes a captured response/status report to the values the app showed
- out-of-range values are rejected

`cargo test -p <id>-proto` must pass before touching hardware.

## Step 5: Driver

Copy `templates/driver.rs` to `crates/periph-app/src/drivers/<id>.rs`, add
`<id>-proto = { path = "../<id>-proto" }` and `hidapi.workspace = true` (if the driver does its
own I/O) to `crates/periph-app/Cargo.toml`, and register it in `drivers/mod.rs`.

1. Implement `open` + `read_config` only. Ask the user to close the vendor app, run the app
   (`npm --prefix app run build` then `cargo run -p periph-app`), and compare every value with
   the vendor app.
2. Add one write command at a time. After each: change it in Periph, close Periph, open the
   vendor app, confirm the same value is shown. Then the reverse direction.
3. `poll` for battery/live status if the device sends status reports; `tray` + `tray_command`
   if quick toggles make sense (see `drivers/vt2.rs`).
4. `open()` errors are shown to the user: make them short and in Portuguese
   ("não encontrado", "o mouse não respondeu (dormindo?)").

## Step 6: UI module

Copy `templates/module/` to `app/src/devices/<id>/` (rename ids and names). One tab per group
of settings; reuse `$lib/components/ui/*`, `$lib/components/page-header.svelte` and
`setting-row.svelte`; look at `devices/vt2/pages/` for patterns (sliders with apply buttons,
toggle groups, confirmation before risky remaps like the primary button). UI copy in pt-BR.

- `demo.state` must be realistic (use values from the capture) and `demo.apply` must mirror
  every command, so the whole UI works at `http://localhost:5173/?demo`.
- Optional: `Card` (sidebar), `Badges` (title bar), `Layer` (background art), `waiting`.
- Vendor renders: `assets/` + `NOTICE.md` from the template, loaded with `import.meta.glob`,
  with a placeholder when missing. Never commit them.

Verify with the dev server (`.claude/launch.json` has `ui-dev`): every tab renders in
`?demo`, commands update the demo state, no console errors, and the module still renders with
its `assets/` folder empty.

## Step 7: Finish

Run the checklist at the end of `docs/adding-a-device.md`: `cargo test --workspace`,
`npm --prefix app run build`, real-device round trips, protocol doc, README device list. Tell
the user exactly what was verified on hardware and what was only verified in demo mode.
