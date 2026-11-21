# Device packages

A **device package** adds support for a peripheral without rebuilding Periph. It is a folder of
JSON files that the app loads at startup:

```
<id>/
  manifest.json        identity, which USB device it may open (what the user reviews)
  device.json          protocol, settings, status, tray, pages, examples, demo data
  locales/en.json      every label (required), plus one file per extra language
  locales/pt-BR.json
```

Periph looks for packages in:

- `%APPDATA%\Periph\devices\<id>\` (the user's packages, see `crates/periph-app/src/packages.rs`)
- every folder listed in the `PERIPH_PACKAGES` environment variable (`;`-separated on Windows),
  for package development

A package that can't be loaded is skipped, and Periph shows why when it starts.

Examples: `packages/rapoo-vt2-max` (registers transport) describes the same mouse as the
built-in `vt2` driver, is tested to produce the same write commands and was verified on the
mouse; `crates/periph-pack/tests/fixtures/sample-reports` (reports transport) is a made-up
device used by the tests.

The AI-agent workflow for writing a package from scratch is the `add-device` skill
(`.claude/skills/add-device/SKILL.md`). The release kit ships it with `periph-pack.exe`.

## periph-pack

The toolbox for package authors. Nothing in it writes to a device.

```
periph-pack probe                                  list HID devices
periph-pack probe --vid 24AE [--pid 1423]          interfaces and report layouts
periph-pack probe --vid 24AE --listen 10           what the device sends on its own
periph-pack new <folder> --vid .. --pid .. --usage-page .. --usage .. [--name ..] [--kind mouse] [--transport registers|reports]
periph-pack check <folder>...                      validate and replay the examples
periph-pack dir                                    the user's package folder
```

From a clone: `cargo run -p periph-pack -- <command>`. `check` validates both files, checks that
every key the package uses exists in every language, and replays the `examples` (captured
commands) against a simulated device.

Preview the generated UI without hardware: `npm --prefix app run dev`, then open
`http://localhost:5173/?demo&packages` (it loads every package in `packages/` with its `demo` data).

## Two layers

| | layer 1: declarative | layer 2: script (planned) |
| --- | --- | --- |
| what you write | `device.json` | `device.json` + `driver.js` |
| covers | register maps, one report per setting, checksums (sum, xor, crc8/16/32), fixed codes, ranges, lookup tables, bit flags | encryption, multi-step handshakes, state machines |
| runs | the built-in engine (`crates/periph-pack`) | QuickJS embedded in the Rust host, with the same HID allowlist |
| UI | generated from the settings | generated, or an optional `ui.js` module |

Layer 1 is what this document specifies and what `crates/periph-pack` implements. Layer 2 reuses
everything here: the script only replaces the encode/decode of settings, and every write still
goes through the host, which enforces the manifest.

## Safety model

The engine, not the package author, enforces the rules in `AGENTS.md`:

- It opens only the HID interfaces the manifest matches (VID, PID, usage page, usage).
- It has no "send raw bytes" command. Every write is a template from `device.json` filled with a
  value that passed the setting's validation (`values`, `range`, codec variants).
- Settings marked `readOnly`, and codec variants marked `readOnly` (decoded but never observed
  being written), are rejected on write.
- Register reads are limited to `transport.readable`; `reports` packages only send the requests
  they declare (`reads`, `send`, `afterWrite`).
- `examples` hold bytes captured from the official software. `periph-pack check` replays them and
  the community registry will require them: a package whose writes are not backed by an example
  does not get merged.

What a package can never do: install drivers, talk to another device, or reach files, network
or processes. That is why some devices stay built in (see "Built-in drivers" below).

## manifest.json

```json
{
  "format": 1,
  "id": "rapoo-vt2-max",
  "name": "Rapoo VT2 Max",
  "kind": "mouse",
  "version": "0.1.0",
  "authors": ["..."],
  "description": "DPI, polling rate, sensor and buttons.",
  "replaces": "vt2",
  "products": [
    { "vid": "0x24AE", "pid": "0x4623" },
    { "vid": "0x24AE", "pid": "0x1423" }
  ],
  "interfaces": {
    "command": { "usagePage": "0xFF00", "usage": "0x000E" },
    "status": { "usagePage": "0xFF00", "usage": "0x0002", "optional": true }
  }
}
```

- `id`: lowercase letters, digits and `-`; it is the translation namespace and must not clash
  with a built-in driver.
- `kind`: `mouse`, `keyboard`, `headset`, `controller` or `other`.
- `replaces`: optional id of a built-in driver this package supersedes. That driver is not started
  while the package is installed, so both never fight over the device.
- `products`: tried in order; the first one present wins (VT2: cable before dongle).
- `interfaces`: named HID interfaces (top-level collections) of that product. `device.json`
  refers to them by name.

Numbers can be written as JSON numbers or as strings (`"0x24AE"`, `"36"`).

## device.json

### Bytes: what you send and what you get

- What Periph **sends** is written in full, report id first (`"06 A5 A4 ..."`), and zero-padded
  to `reportLength`. Checksum positions count that report id as byte 0.
- What the device **answers** (responses, read answers, status reports) is addressed *after* the
  report id: offset 0 is the first byte after it. Each answer names its `reportId`; the engine
  strips it (hidapi returns some answers with it and some without).

### transport: registers

The device exposes a memory map that is read and written with request/response reports. A common
design in vendor mice and keyboards (VT2, many Compx/Pixart-based mice).

```json
"transport": {
  "type": "registers",
  "interface": "command",
  "reportLength": 33,
  "read":  "06 A5 A4 {len} {addr:u16le} 00 00",
  "write": "06 A5 A5 {len} {addr:u16le} 00 00 {data}",
  "maxChunk": 20,
  "writeBlock": 4,
  "readable": [{ "from": "0x0600", "to": "0x08FF" }],
  "response": {
    "reportId": 6,
    "status": 0,
    "data": 4,
    "ready": [1],
    "busy": [2],
    "pending": [0],
    "pendingCode": "asleep",
    "timeoutMs": 1000
  }
}
```

- `read`, `write`: byte templates. Hex bytes are literals; `{len}` is the chunk length (u8),
  `{addr:u16le}` / `{addr:u16be}` / `{addr:u8}` the address, `{data}` the chunk. Requests are
  zero-padded to `reportLength` and sent as output reports.
- `maxChunk`: longer reads and writes are split, like the official app does.
- `writeBlock`: a setting without its own `write` blocks is written read-modify-write: read
  `writeBlock` bytes at its address, patch them, write them back.
- `response`: the answer is fetched with *get input report* `reportId`; `status` and `data`
  are offsets after the report id. `ready`/`busy`/`pending` list status byte values; busy and
  pending are retried until `timeoutMs`; a pending timeout reports the device as absent with
  `pendingCode` (the UI explains `asleep` and `notFound`; other codes show the generic text).

### transport: reports

Each setting is written with its own request, filled from a template; the state is fetched with
the requests listed in `reads`. Common in gaming mice, headsets and keyboards with their own
protocol.

```json
"transport": {
  "type": "reports",
  "interface": "command",
  "send": "feature",
  "reportLength": 65,
  "checksum": { "type": "crc8", "from": 1, "to": -2, "at": -1 },
  "afterWrite": ["04 0F 01"],
  "delayMs": 5
},
"reads": [
  {
    "id": "config",
    "request": "04 01",
    "response": { "from": "feature", "reportId": 4, "length": 65, "match": [{ "at": 0, "equals": 1 }], "timeoutMs": 500 }
  }
]
```

- `send`: `output` (default) or `feature` reports.
- `checksum` (optional): `sum8` (with `"negate": true` for 0x100 − sum), `xor8`, `crc8` (poly
  0x07), `crc16ccitt` (0x1021, init 0xFFFF), `crc32` (zlib). It covers bytes `from`..`to` and is
  written at `at` (little endian unless `"bigEndian": true`); negative positions count from the
  end. `prefix` adds bytes in front of the data (e.g. `"A2"`, DualSense over Bluetooth).
- `afterWrite`: fixed requests sent after every write (a "save" command, if the official app
  sends one).
- `reads`: each request's answer is kept and settings point into it. `from: "feature"` gets the
  feature report `reportId`; `from: "input"` waits for the next input report with that id that
  passes `match`.

Settings of a `reports` package:

```json
{ "id": "dpi", "from": "config", "at": 1, "type": "u16le", "range": { "min": 100, "max": 16000, "step": 50 }, "send": "04 02 {value}" }
{ "id": "colors", "from": "config", "at": 5, "type": "u8", "count": 2, "range": { "min": 0, "max": 255 }, "send": "04 05 {index} {value}" }
{ "id": "sleep", "type": "u8", "range": { "min": 1, "max": 30 }, "default": 10, "send": "04 06 {value}" }
```

- `from` + `at`: where the value is in a read's answer. Without `from` the setting is
  write-only: it shows `default`, then the last value written.
- `send`: one template or a list of them. `{value}` is the encoded value (its width comes from
  `type`), `{index}` the item of an array. A setting without `send` is read-only.
- A `bit` setting needs `from`: the other bits of its byte are written back unchanged.

A `script` transport (layer 2) is planned for what neither covers.

### settings

Every setting is a value in the config the UI edits:

```json
{ "id": "polling", "at": "0x0880", "type": "u8",
  "values": [ { "value": 125, "raw": "0x08" }, { "value": 1000, "raw": "0x01" } ] }
```

| key | meaning |
| --- | --- |
| `id` | name in the config and in commands |
| `at` | registers: address, or a list of addresses that always hold the same value (mirrors); reports: offset in the answer of `from` |
| `from`, `send`, `default` | reports transport, see above |
| `type` | `u8`, `i8`, `u16le`, `u16be`, `bool` (non-zero), `bit` (with `bit`), or a codec name |
| `bit`, `invert` | for `bit`: bit number; `invert` when 1 means off |
| `count` | the setting is an array of `count` items, one after the other |
| `values` | lookup table raw ↔ value; only listed values can be written |
| `linear` | `{ "scale": 0.1, "offset": 0.6 }`: value = raw × scale + offset |
| `range` | `{ "min", "max", "step" }` allowed values (after `linear`) |
| `unit` | shown next to the value |
| `write` | registers: explicit read-modify-write blocks: `[{ "at", "len" }]` |
| `readOnly` | shown, never written |
| `indexOf` | the value is an index into another array setting (e.g. the active DPI stage) |
| `countFrom` | only the first N items of this array are used, N = another setting's value |
| `control` | UI hint: `switch`, `segmented`, `select`, `slider`, `number`, `list`, `stages` |

### codecs

Settings that are not a plain number use a codec: a list of byte patterns, each decoding to
`{ "kind": ..., "value": ... }`.

```json
"codecs": {
  "action": {
    "size": 4,
    "variants": [
      { "kind": "mouse", "bytes": "03 00 {value} 00", "values": [1, 2, 4, 8, 16] },
      { "kind": "key", "bytes": "00 00 {value} 00", "range": { "min": 4, "max": 115 } },
      { "kind": "dpiUp", "bytes": "08 00 05 00" },
      { "kind": "wheel", "bytes": "07 00 {value} 00", "readOnly": true },
      { "kind": "disabled", "bytes": "FF FF FF FF" }
    ]
  }
}
```

Bytes that match no variant decode to `{ "kind": "raw", "bytes": "..." }` and cannot be written.

### status

Live values the device reports on its own (battery, current DPI):

```json
"status": {
  "interface": "status",
  "reportId": 7,
  "match": [ { "at": 0, "mask": "0xF0", "equals": "0x20" } ],
  "fields": [
    { "id": "battery", "at": 7, "type": "u8", "unit": "%" },
    { "id": "charging", "at": 6, "type": "bool", "equals": 2 }
  ]
}
```

Status fields use the same types as settings; `at` is an offset after the report id. The status
may come on its own interface or on the transport's.

### tray

```json
"tray": {
  "battery": { "level": "battery", "charging": "charging" },
  "header": "tray.header",
  "items": [
    { "setting": "active", "label": "tray.dpi" },
    { "setting": "polling" },
    { "setting": "motionSync" },
    { "refresh": true }
  ]
}
```

`header` is a translation key; `{name}` placeholders are filled with status fields. A `bool` or
`bit` setting becomes a switch, a setting with `values` or `indexOf` a choice group.

### pages

The UI is generated from the settings:

```json
"pages": [
  { "id": "dpi", "icon": "dpi", "settings": ["dpi", "active"] },
  { "id": "sensor", "icon": "sensor", "settings": ["motionSync", "lod"] }
]
```

### examples

Captured commands, replayed by `periph-pack check` against a simulated device:

```json
"examples": [
  {
    "memory": { "0x0880": "08 06 FF FF" },
    "set": { "id": "polling", "value": 1000 },
    "writes": ["06 A5 A5 04 80 08 00 00 01 06 FF FF"]
  }
]
```

`writes` lists the write requests in order (read requests are not compared; they never change
the device). `memory` is the register map before the command (registers); for `reports`,
`responses` gives each read's answer before the command (`{ "config": "01 90 01 01 00 10 20" }`,
after the report id), and `writes` includes the `afterWrite` requests.

### demo

`"demo": { "config": {...}, "status": {...} }` is what `npm --prefix app run dev` (`?demo`)
shows, so the UI can be designed without the device.

## Translations

`locales/<lang>.json` is namespaced by the package id, exactly like a built-in module. Keys the
generated UI looks up:

- `name` (optional, the device name)
- `pages.<page>`: tab label
- `settings.<id>.label`, `settings.<id>.description` (optional)
- `settings.<id>.options.<value>` (optional label for a `values` entry)
- `settings.<id>.items` (array) or `settings.<id>.item` (`"Stage {n}"`): labels of array items
- `codecs.<codec>.values.<kind>.<value>`: label of a codec value
- `codecs.<codec>.<kind>`: label of a codec variant
- `tray.*`: whatever `tray` names

## Commands (UI → engine)

```
{ "type": "refresh" }
{ "type": "set", "id": "polling", "value": 1000 }
{ "type": "set", "id": "dpi", "index": 2, "value": 1600 }
{ "type": "set", "id": "buttons", "index": 6, "value": { "kind": "key", "value": 4 } }
```

## Built-in drivers

Packages cover devices that are configured over HID. Some devices need more than that and stay
built in, compiled into Periph:

- **DualSense.** Lights and adaptive triggers are plain HID output reports and could be a package
  (with a CRC32 primitive for Bluetooth). Emulation is not: it needs the `periph-vpad` kernel
  driver, HidHide and a virtual controller, which a sandboxed package must never install or
  drive. So the DualSense keeps its built-in driver.
- Anything else that needs system integration (drivers, services, input injection).

Later, the host can expose privileged features as **services** that a package asks for in its
manifest and the user approves on install, for example `"services": ["virtualController"]`.
The host keeps ownership of the kernel driver; the package only describes the device and its
mapping.
