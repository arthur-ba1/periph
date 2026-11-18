# Adding a device to Periph

Periph is a shell. Each peripheral is a **device implementation** made of two halves that share
an id (`vt2`, `via`, `mydevice`, ...):

| half | where | what it does |
| --- | --- | --- |
| driver (Rust) | `crates/periph-app/src/drivers/<id>.rs` (+ optional protocol crate) | opens the device, reads its config, applies commands, polls status |
| module (Svelte) | `app/src/devices/<id>/` | tabs/pages, sidebar card, title-bar badges, demo data |

The shell discovers both automatically once registered: the device picker, sidebar, tray, worker
thread, reconnection, busy/disconnect handling and toasts are generic.

```
UI page ──send(id, cmd)──▶ device_send ──▶ worker thread ──▶ Session::apply ──▶ USB/HID
   ▲                                             │
   └──── "device" event {device:id, type, ...} ◀─┘  read_config / poll
```

If you are an AI agent, follow the skill in `.claude/skills/add-device/SKILL.md`: it covers the
whole process, from identifying the device to reverse engineering the vendor protocol.

## 1. Is it already supported by a standard?

- **QMK/VIA keyboard**: no code needed. Drop its VIA definition JSON into
  `app/src/devices/via/keyboards/` (the Rust driver embeds every file there at build time and
  the UI picks it up). Get the JSON from the keyboard vendor or the `the-via/keyboards` repo.
  Check the file's license before committing it here (this repo is MIT); a definition you only
  use yourself can stay local and untracked, it is picked up all the same.
- Anything else (vendor mice, headsets, non-VIA keyboards, controllers): write a driver.

## 2. Driver (Rust)

Contract: `crates/periph-core/src/lib.rs`.

```rust
pub trait Driver: Send + Sync + 'static {
    fn info(&self) -> DeviceInfo;                                     // { id, name, kind }
    fn open(&self) -> anyhow::Result<(Box<dyn Session>, Connected)>; // Err = absent; message shown in UI
    fn retry_interval(&self) -> Duration { 2s }
    fn tray(&self, snap: &Snapshot) -> Option<Tray> { None }          // tray header, battery, quick items
    fn tray_command(&self, id: &str, snap: &Snapshot) -> Option<Value> { None }
}

pub trait Session: Send {
    fn read_config(&mut self) -> anyhow::Result<Value>;  // whole editable config, after connect and after every command
    fn apply(&mut self, cmd: Value) -> anyhow::Result<()>;
    fn poll(&mut self) -> anyhow::Result<Option<Value>> { Ok(None) } // live status; Err = device gone
    fn poll_interval(&self) -> Duration { 250ms }
}
```

Conventions (copy `drivers/vt2.rs`, or the template in the skill):

- Commands are a typed enum, decoded with `periph_core::parse::<Cmd>(cmd)`:
  ```rust
  #[derive(Deserialize)]
  #[serde(tag = "type", content = "value", rename_all = "camelCase")]
  enum Cmd { Refresh, Polling(u32), StageDpi((usize, u16)) }
  ```
  The UI sends `{ type: "polling", value: 1000 }`. Always include `Refresh` (the title-bar
  button sends `{ type: "refresh" }`).
- Config/status are typed structs with `#[serde(rename_all = "camelCase")]`, serialized with
  `serde_json::to_value`.
- Keep wire-format code out of the driver: put encoding/decoding in a pure crate
  (`crates/<id>-proto`, no I/O, unit-tested against captured bytes) and HID I/O in a transport
  (`crates/<id>-device` or a small module). See `vt2-proto` / `vt2-device`.
- Register it in `crates/periph-app/src/drivers/mod.rs`.

### Safety rules (non-negotiable)

1. Only send byte sequences you have **observed the official software send**. Each write
   command maps to a captured example and a unit test.
2. `apply` rejects values outside the ranges the official software allows (an allowlist, not a
   blocklist). Unknown or partially decoded fields are read-only in the UI.
3. Read-modify-write when the official app does; never write a whole block you only partly
   understand.
4. Never send firmware update, bootloader, factory-reset or calibration commands, and never
   fuzz write commands.
5. Discovery tools only read (`vt2-probe`, `via-probe`).

## 3. Module (Svelte)

`app/src/devices/<id>/index.js` default-exports:

```js
export default {
  id: "mydevice",               // = Rust DeviceInfo.id
  name: "Brand Model",
  kind: "Mouse",                // label under the name in the device picker
  icon: "mouse",                // name from src/lib/icons/icons.js
  order: 3,                     // position in the picker
  tabs: [{ label: "DPI", icon: "dpi", page: DpiPage }],
  // optional:
  Card,                         // sidebar card component (default: icon + name + status)
  Badges,                       // title-bar badges component
  Layer,                        // background layer, gets { shellEl, mainEl, tab } (see vt2/Hero.svelte)
  displayName: (dev) => "...",  // dynamic name (VIA uses the definition name)
  ready: (dev) => !!dev.config, // when to show pages instead of the waiting screen
  waiting: (dev) => ({ title, text, hideReason }), // waiting screen copy
  demo: { state: { product, serial, config, status }, apply(dev, cmd) {} }, // browser demo (?demo)
};
```

State and commands, from `$lib/device-state.svelte.js`:

```js
import { deviceState, send } from "$lib/device-state.svelte.js";
export const dev = deviceState("mydevice"); // { connected, product, serial, reason, config, status, busy, driver }
send("mydevice", { type: "polling", value: 1000 });
```

Put these in a small `store.svelte.js` in the module (see `devices/vt2/store.svelte.js`) and
import it from the pages. Pages are plain Svelte 5 components; reuse the shadcn-svelte
components in `$lib/components/ui/` and `$lib/components/{page-header,setting-row}.svelte` so
the device looks like the rest of the app. UI copy is in Portuguese (pt-BR).

Folders starting with `_` are ignored by the registry.

### Demo mode

`npm --prefix app run dev` and open `http://localhost:5173/?demo`: every module is shown as
connected with its `demo.state`, and `send()` calls `demo.apply(dev, cmd)` instead of the
backend. Every module must have a demo so the UI can be developed and reviewed without the
hardware.

## 4. Vendor artwork

Product renders from the official software are copyrighted. Never commit them. Put them in
`app/src/devices/<id>/assets/` (git-ignored except `NOTICE.md`), load them with
`import.meta.glob` so a missing file is not a build error, show a neutral placeholder when they
are absent, and document in `assets/NOTICE.md` how a user regenerates them from their own copy
of the vendor software. Your own drawings (SVG made from scratch) can be committed elsewhere in
the module.

## 5. Checklist

- [ ] `cargo test --workspace` passes, including protocol tests built from captured bytes
- [ ] `npm --prefix app run build` passes
- [ ] `?demo` shows every tab working, and the app works with the assets folder empty
- [ ] tested on the real device: connect, every setting round-trips (change, refresh, still
      there), unplug/replug, sleep/wake
- [ ] `docs/<id>-protocol.md` describes transport, command format and memory map, with the
      capture each fact came from
- [ ] the device is listed in the README
