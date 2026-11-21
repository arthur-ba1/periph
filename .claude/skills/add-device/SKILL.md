---
name: add-device
description: Make Periph configure a peripheral it does not support yet (mouse, keyboard, headset, controller...) by writing a device package, a folder of JSON files the app loads at runtime, from identifying the device and capturing its vendor software's HID traffic to a checked, working package. Use when someone wants Periph to support their own device, to add a VIA keyboard definition, to extend a package with settings that are not decoded yet, or (rarely) to write a built-in Rust driver.
---

# Add a device to Periph

The goal is a **device package**: `manifest.json` + `device.json` + `locales/`, in the user's
package folder. Periph loads it at startup; nothing is compiled. The format reference is
`device-packages.md` and the capture method is `reverse-engineering.md` (see "Where things
are"). Read the format reference before writing any JSON.

The user owns the hardware and the vendor app. Many steps need them to plug something in, open
the vendor app or change a setting while you capture. Ask for exactly one action at a time and
say what you will do with it. Talk to them in their language; keep the package files in English
except the translations.

## Where things are

Find out which layout you are in before step 1:

| | Periph repository (a clone) | release kit (zip from the Releases page) |
| --- | --- | --- |
| toolbox | `cargo run -q -p periph-pack -- <cmd>` | `periph-pack.exe <cmd>` (kit root) |
| capture tools | `tools/hidhook.py`, `tools/summarize.py` | `tools/` next to this file |
| format reference | `docs/device-packages.md` | `docs/device-packages.md` next to this file |
| capture method | `docs/reverse-engineering.md` | `docs/reverse-engineering.md` next to this file |
| examples | `packages/rapoo-vt2-max` (registers), `crates/periph-pack/tests/fixtures/sample-reports` (reports) | `examples/` next to this file |
| the app | `target/release/periph.exe` | `Periph.exe` (kit root, or installed) |

Below, `periph-pack` means the toolbox command of your layout. `periph-pack dir` prints the
user's package folder (`%APPDATA%\Periph\devices`).

## Safety (applies to every step)

- Never send a byte sequence to the device that you have not seen the official software send.
  No guessing, no fuzzing, no "let's try opcode 0xA6". The package's `examples` are the proof:
  every write template must be backed by a captured example.
- Never describe firmware update, bootloader, factory reset, pairing or calibration commands.
- `periph-pack probe` only reads. Before the package's first write is tried on hardware, its
  read path must already match what the vendor app shows.
- Writable values are allowlists (`values`, `range`, codec variants) limited to what the
  official app offers. Anything not understood stays `readOnly`.
- If something is ambiguous, stop and ask the user rather than experiment on their hardware.
- Do not put vendor artwork, binaries or raw captures with serial numbers in a package.

## Step 0: Gather facts

Ask (or find out) and write down: brand and exact model; wired / wireless dongle / Bluetooth;
the official configuration app and whether it is installed here; which settings they care about.

Then choose the path:
- **QMK/VIA keyboard** (works with usevia.app): no package needed. Get its VIA definition JSON
  (vendor or the `the-via/keyboards` repo) and put it in Periph's `devices/via/keyboards/`
  folder (repository: `app/src/devices/via/keyboards/`, then rebuild). Done.
- **A device another open project documents** (OpenRazer, libratbag/Piper, rivalcfg,
  HeadsetControl, ...): use their notes as a hypothesis, but still confirm every write against
  a capture of the official app.
- **Needs more than HID configuration** (a kernel driver, a virtual device, input injection,
  like the DualSense emulation): that is a built-in driver; see "Built-in driver" at the end.
- **Otherwise**: a package, below.

Pick the id now: lowercase letters, digits and `-`, short and stable (`g502x`, `viper-v3-pro`).
It is the package folder name and its translation namespace.

## Step 1: Identify the device (read-only)

Device plugged in, vendor app closed:
```
periph-pack probe                          # find its VID:PID in the list
periph-pack probe --vid <VID> --pid <PID>  # interfaces and report layouts
periph-pack probe --vid <VID> --listen 15  # what it sends on its own (move it, wait for battery)
```
Note every PID (wired and dongle differ), the vendor-defined interfaces (usage page 0xFF00 or
above: configuration almost always happens there), their report ids, kinds and lengths.

Create the package skeleton in a working folder (not yet the user's package folder):
```
periph-pack new <work>/<id> --vid <VID> --pid <PID> --usage-page <page> --usage <usage> --name "<Brand Model>" --kind mouse
```
Start a `<work>/<id>/NOTES.md` and keep every decoded fact there with the marker it came from.

## Step 2: Capture the official app

Follow sections 2 and 3 of `reverse-engineering.md`. Needs Python and `pip install frida-tools`
(ask before installing). Find whether a vendor DLL exports hidapi, then:
```
python tools/hidhook.py <work>/capture.jsonl --process "<Vendor App>.exe" --module "<dll or empty>"
```
Before each change in the vendor app, append a marker line to the marker file
(`echo dpi 800 >> captures/marker.txt`, path printed by hidhook). If you have computer-use
tools you may drive the vendor app yourself, still one change per marker. Capture: app startup
(it reads everything), idle, each setting at 2-3 values, every value of each enumeration, a few
button remaps, unplug/replug, sleep/wake. Write down what the app showed for each marker.

## Step 3: Decode and choose the transport

`python tools/summarize.py <work>/capture.jsonl --since-marker "<text>"` (it collapses busy
polls of the VT2; on other devices read the raw lines too). Work out, in order: framing (report
id, length, padding, opcodes), the answer handshake, the reads done at startup, then each
setting's encoding (units, endianness, bit flags, lookup tables), status reports, and whether
the app writes read-modify-write or one value per report. Check every captured value against
your formula. Unexplained bytes stay unknown and read-only.

Then pick the transport (`device-packages.md` explains both):
- **registers**: the app reads and writes blocks of a memory map with address + length
  requests (VT2 style). Model settings as addresses; writes become read-modify-write of blocks.
- **reports**: each setting is sent as its own report from a template, often with a checksum;
  the state comes from one or more read requests (or can't be read at all: write-only settings
  with a `default`).
- Neither fits (encryption, multi-step handshakes, checksums the engine lacks): stop and tell
  the user; that needs engine support (an issue or PR on the Periph repository) or a built-in
  driver.

Show the user the decoded table and confirm values against the vendor app before step 4.

## Step 4: Write the package

Fill `device.json` following `device-packages.md` and the examples:
1. `transport` and `reads` (reports) exactly as captured.
2. `settings`: start with the ones the user cares about. Every writable number needs `values`
   or `range` taken from what the vendor app offers. Use `codecs` for multi-byte actions
   (button mappings); mark variants you only saw decoded as `readOnly`.
3. `status` + `tray.battery` if the device reports battery on its own.
4. `examples`: one per write template, with the captured bytes as `writes` and the state
   before the change in `memory` (registers) or `responses` (reports). Paste hex from the log.
5. `pages`, `tray.items`, and `locales/en.json` plus every other language the user wants
   (translate yourself; Periph ships English and Brazilian Portuguese). Labels:
   `settings.<id>.label`, `.description`, `.options.<value>`, `.items`, `pages.<page>`,
   `codecs.<codec>.<kind>`.
6. `demo` with realistic values from the capture.

Then:
```
periph-pack check <work>/<id>
```
It must print `ok` with no warning. Fix every problem it lists; never weaken a `range` or
remove an example to make it pass.

## Step 5: Try it on the device

1. Copy `<work>/<id>` into the user's package folder (`periph-pack dir`). To try a package
   without installing it, start Periph with `PERIPH_PACKAGES=<work>` instead.
2. Close the vendor app, start Periph. If the package has a problem, Periph shows a "was not
   loaded" message with the reason. Compare every value on screen with the vendor app.
3. One write at a time: change it in Periph, close Periph, open the vendor app and confirm it
   shows the same value. Then put it back.
4. Battery and tray: check the sidebar card and the tray flyout.

Tell the user exactly what was verified on hardware and what was only checked by
`periph-pack check`.

## Step 6: Share it (optional)

A package is a folder: the user can zip it and share it. To have it listed for everyone, open
a pull request adding it under `packages/<id>/` in the Periph repository with its `NOTES.md`
(captures stay out: they can contain serial numbers).

## Built-in driver (only when a package can't do it)

Some devices need system integration a package must never have (kernel drivers, virtual
devices, input injection). Those are built into Periph (Rust + Svelte) and need a clone of the
repository. Follow `docs/adding-a-device.md` there: a protocol crate from `templates/proto/`, a
driver from `templates/driver.rs` registered in `crates/periph-app/src/drivers/mod.rs`, and a UI
module from `templates/module/`, with the same safety rules, captures and tests. Commits carry
no `Co-Authored-By:` or "Generated with Claude Code" lines.
