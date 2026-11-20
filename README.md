# Periph

**English** · [Português (Brasil)](README.pt-BR.md)

An open app (Rust + Tauri 2 + Svelte 5) to configure computer peripherals without the
manufacturer's software. Each device is a pluggable implementation: a Rust driver plus a Svelte
UI module. Anyone can add their own mouse, keyboard or other peripheral.

## Supported devices
| Device | Id | What you can configure |
| --- | --- | --- |
| **Rapoo VT2 Max** (USB `24AE:1423` dongle, `24AE:4623` cable) | `vt2` | DPI stages, polling rate, lift-off distance, motion sync, linear/ripple correction, glass mode, debounce, buttons, battery |
| **VIA keyboards** (e.g. AKKO 5075) | `via` | per-layer keymap (with knob), RGB lighting |

For a new VIA keyboard, drop its JSON definition into `app/src/devices/via/keyboards/` (the
repository only ships an example definition; check the license before committing another one).

## Adding a device
- Guide: [docs/adding-a-device.md](docs/adding-a-device.md) (driver and module contract)
- Reverse-engineering method: [docs/reverse-engineering.md](docs/reverse-engineering.md)
- For AI agents (Claude Code, Codex, ...): the skill at
  [.claude/skills/add-device/SKILL.md](.claude/skills/add-device/SKILL.md), with ready templates.
  See also [AGENTS.md](AGENTS.md).

## Languages
The UI, the tray and the messages are available in English and Brazilian Portuguese. The
language follows the system and can be changed in the title bar. To add a language, see
[docs/translating.md](docs/translating.md).

## Layout
- `crates/periph-core`: the `Driver`/`Session` contract, the per-device worker loop, i18n
- `crates/periph-app`: the Tauri app (`periph.exe`); drivers in `src/drivers/`, generic tray
- `crates/vt2-proto`, `crates/vt2-device`: VT2 Max protocol and transport ([docs/protocol.md](docs/protocol.md), in Portuguese)
- `crates/via`: VIA protocol
- `crates/vt2-probe`: read-only HID discovery; `crates/vt2-cli`: VT2 Max command line
- `app/src/devices/<id>/`: UI modules, discovered automatically
- `tools/`: Frida HID hook, log summarizer, resource extractors

## Build
```
npm --prefix app install
npm --prefix app run build            # builds app/dist (embedded in the exe)
cargo build --release -p periph-app   # target\release\periph.exe
target\release\periph.exe --tray      # start in the tray only
npm --prefix app run dev              # UI only, no hardware: http://localhost:5173/?demo
cargo test --workspace
```
- Closing the window only hides it to the tray. Each connected device gets its own tray icon with its battery: left click opens its quick settings, double click opens it in Periph and right click shows the menu.
- Close the official software before using Periph: both can't use the HID channel at the same time.
- Icon: `crates/periph-app/app-icon.svg`. To regenerate: `npx --prefix app tauri icon crates/periph-app/app-icon.svg -o crates/periph-app/icons`

## Contributing
- Enable the repository hook once per clone: `git config core.hooksPath .githooks`. It rejects
  commits containing `Co-Authored-By:` or "Generated with Claude Code"; CI runs the same check.
- Before opening a PR: `cargo test --workspace`, `npm --prefix app run build` and
  `npm --prefix app run check:i18n`.

## Vendor artwork
Product renders belong to their manufacturers and are not in the repository. Every module works
without them (it shows a neutral outline) and explains in `app/src/devices/<id>/assets/NOTICE.md`
how to extract them from your own copy of the official software, for local use.

## VT2 Max command line
```
cargo build --release -p vt2-cli
target\release\vt2 info
target\release\vt2 status
target\release\vt2 polling 1000
target\release\vt2 dpi 2 800
target\release\vt2 button back key:f13
```

## Safety rules
- Only send write commands in the exact format the official app uses (verified by tests against captured bytes).
- No fuzzing of write commands, no firmware, bootloader or factory-reset commands.
- Discovery tools only read.

## To do
- [ ] VT2 Max: macros, factory reset, sleep time (not captured yet)

## License
MIT. See [LICENSE](LICENSE).
