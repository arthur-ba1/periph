# DualSense: protocol reference and Periph driver contract

Status:

| Piece | State |
|---|---|
| `crates/dualsense-proto` | done, unit-tested |
| `crates/dualsense-vpad` | done (Windows channel + mock), unit-tested |
| `crates/hidhide` | done (HidHide control client + mock), unit-tested |
| kernel driver `periph-vpad` | separate repository (KMDF + VHF, imports only `ntoskrnl.exe` + `WDFLDR.SYS`, INF and install script); validated on hardware with test signing |
| `crates/periph-app/src/drivers/dualsense/` | done, registered, unit-tested; validated on the real pad over USB and Bluetooth |
| UI module `app/src/devices/dualsense/` | done |

The virtual controller needs the `periph-vpad` kernel driver, which lives in its own repository
with its build and install instructions. Without it Periph still configures the physical pad
(lights, triggers, battery); only emulation reports the driver as missing.

## Transport (observed)

- USB VID `054C`, PID `0CE6`, interface `MI_03`, usage page `0x01`, usage `0x05`.
- 437-byte USB HID report descriptor, captured read-only with the probe (`captures/`,
  git-ignored).
  Embedded in code at `crates/dualsense-proto/src/descriptor.rs`.
- Reports: input `0x01` = 64 B, output `0x02` = 48 B; feature ids and lengths are in
  `crates/dualsense-proto/src/feature.rs` and in the probe output.

Capture commands (read-only, nothing is ever written by the probe):
```
cargo run -p vt2-probe -- --vid 054c                  # interfaces + descriptor
cargo run -p vt2-probe -- --vid 054c --ds-features    # GET_FEATURE 0x05/0x09/0x20/0x22
```

## Bluetooth (observed)

The pad also works paired over Bluetooth. The HID path then carries the HID service UUID
(`\\?\hid#{00001124-0000-1000-8000-00805f9b34fb}_vid&0002054c_pid&0ce6#...`). The data is the
same as over USB, framed in other report ids with a CRC-32 (zlib polynomial) appended. The CRC
covers a one-byte seed plus the report minus its last 4 bytes, stored little-endian in those
bytes. Source: the official software captured with Frida while the pad connected and effects were
changed (`captures/dsx-bt.jsonl`, git-ignored). Code: `crates/dualsense-proto/src/bluetooth.rs`.

| Report | Layout | CRC seed |
|---|---|---|
| output `0x31`, 78 B | `31 02`, then USB output bytes 1–47 (the USB body moves up by one byte: R2 at 12, L2 at 23, lightbar at 46–48), zeros at 49–73, CRC at 74–77 | `0xA2` |
| feature reports | same ids and lengths as USB; the last 4 bytes are a CRC where USB has zeros | `0xA3` |
| input `0x31`, 78 B | `31`, tag byte, then USB input bytes 1–63, CRC at 74–77 | `0xA1` |

- Byte 1 of the output was `0x02` in every captured report (no sequence counter).
- On connect, the official software reads features 0x20, 0x09, 0x05 and 0x22. Reading 0x05 is
  what switches the pad from its reduced 0x01 input report to the full 0x31 one, as in Linux
  `hid-playstation`. Periph reads 0x05 when it opens a Bluetooth pad. The official software also
  sends test commands (feature 0x80, read back with 0x81) to fetch serial numbers. Periph never
  sends them.
- Windows pads Bluetooth reads and writes to the descriptor's largest report (547 B). hidapi
  zero-pads writes. GET_FEATURE needs a buffer at least that large.
- The input layout and seed come from `hid-playstation`. The capture hook saw no input reads (the
  official software uses overlapped reads). Periph verifies the CRC and ignores any report that
  fails it. Confirmed on the user's pad (2026-11-20): status/battery and emulated input work,
  and a game's adaptive triggers reach the pad over Bluetooth.
- After connecting, the official software sends its state in reports `0x32` (142 B), `0x36`
  (398 B) and `0x39` (547 B). The same CRC scheme applies (seed 0xA2). `0x32` starts with
  `32 <seq<<4> 90 3f` followed by the 47-byte USB body, and these reports also carry the
  Bluetooth haptics stream. Periph uses only `0x31`. The 0x32 family is not decoded and is
  never sent.
- The capture contains no motor rumble over Bluetooth: the official software's vibration test
  drove the trigger vibration mode. Motor bytes sit at the same body offsets as over USB.

## Input report 0x01 (64 bytes) — offsets

| Off | Meaning |
|---|---|
| 0 | 0x01 |
| 1–4 | LX, LY, RX, RY (0..255, center 0x80) |
| 5, 6 | L2, R2 analog |
| 7 | sequence/timestamp byte |
| 8 | dpad low nibble (0–7 = N..NW, 8 = released) \| Square 0x10, Cross 0x20, Circle 0x40, Triangle 0x80 |
| 9 | L1 0x01, R1 0x02, L2 0x04, R2 0x08, Share 0x10, Options 0x20, L3 0x40, R3 0x80 |
| 10 | PS 0x01, touchpad click 0x02, mute 0x04 |
| 11 | packet counter |
| 12–15 | timestamps |
| 16–27 | gyro XYZ + accel XYZ (i16 LE) |
| 28–31 | timestamp (µs) |
| 32 | constant 14 |
| 33–41 | touchpad: 2 × (tracking id + 3 bytes) + packet counter at 41 |
| 42, 43 | adaptive trigger R/L state echo |
| 48 | trigger state |
| 53 | battery (low nibble = level, high = state) |
| 54 | constant 8 |

The driver forwards the whole 64-byte frame (remap/deadzone are applied on top later), so fields
marked as constants/reserved are preserved. Encoding: `dualsense-proto/src/input.rs`.

## Output report 0x02 (48 bytes) — what the game writes

| Off | Meaning |
|---|---|
| 0 | 0x02 |
| 1 | valid_flag0: bit0 compatible vibration, bit1 haptics select (either one makes bytes 3–4 apply), bit2 R trigger, bit3 L trigger |
| 2 | valid_flag1: bit0 mic LED, bit2 lightbar, bit3 release LEDs, bit4 player LEDs, bit6 motor strength |
| 3, 4 | right / left rumble motor |
| 9 | mic LED (0 off, 1 on, 2 breathing) |
| 11–21 | right adaptive trigger (mode + 10 params) |
| 22–32 | left adaptive trigger (mode + 10 params) |
| 37 | motor strength flags |
| 39 | valid_flag2: bit0 brightness valid, bit2 improved rumble |
| 43 | player/mute brightness |
| 44 | player LED bitmask |
| 45–47 | lightbar R, G, B |

Same layout as Linux `hid-playstation`. Every section is applied by the controller only when its
valid-flag bit is set, so games send partial updates (a lightbar-only report leaves the motors
alone). The Periph driver forwards every frame in order and runs a 5-second rumble watchdog:
it starts on a report that sets the rumble bits with a non-zero motor, is cancelled by one that
sets them with both motors at 0, and ignores reports without the rumble bits. On expiry it sends
`OutputReport::rumble_stop()` (only bit0 of byte 1 set, motors 0, nothing else flagged), so
lightbar, LEDs and triggers are never rewritten with stale values.

## Feature reports (answered by the virtual device)

Allowlist (id, length incl. id): `0x05` = 41 B (calibration), `0x09` = 20 B (MAC),
`0x20` = 64 B (firmware info), `0x22` = 64 B (BT patch info).
Responses come from GET_FEATURE reads of the user's own physical controller
(`--ds-features`); they are loaded at runtime and never hardcoded with another unit's data.
Factory/test reports (0x80+, 0xF0+) are **never** fabricated or forwarded as writes.

Example capture (real, this machine): `crates/dualsense-proto/src/feature.rs` test
`parse_probe_capture`.

## Virtual device (own kernel driver)

- KMDF + VHF (`vhfkm.lib`), root-enumerated device with hardware id
  `Root\PeriphVirtualDualSense` (created by the `periph-vpad` install script via
  `devcon install`); the HID child exposes VID_054C&PID_0CE6 and the same 437-byte descriptor.
- User-mode channel: `\\.\PeriphVpad`, custom IOCTLs (`src/ioctl.rs` in the `periph-vpad`
  repository, mirrored in `crates/dualsense-vpad`). Every IOCTL completes immediately:
  - `SUBMIT_INPUT` (64 B in);
  - `READ_OUTPUT`: success with 48 B = oldest queued report, success with **0 B = queue empty**
    (any failure is a real error);
  - `SET_FEATURE`, `GET_STATUS` (bit 0: started), `START`, `DESTROY`.
- Output reports are kept in an 8-entry FIFO (oldest dropped when full), so partial updates
  arriving between two polls are not lost.
- Lifetime: closing the handle that sent `START` removes the virtual pad (EvtFileCleanup), and
  removing the root device deletes it too; a crashed Periph never leaves a frozen controller.
- Locking: one kernel spin lock (`KeAcquireSpinLockRaiseToDpc`); the IOCTL queue is sequential
  and runs at PASSIVE_LEVEL.
- Physical-vs-virtual disambiguation: both carry identical VID/PID and USB-shaped instance
  ids, so the driver walks the **PnP parent chain** (`CM_Locate_DevNodeW` + `CM_Get_Parent`):
  a real controller descends from a `PCI\...` device; an emulated one descends from a
  root-enumerated software bus (`ROOT\...`). Fast-path markers (`periph`, `vhf` in the path)
  catch our own device without a configuration-manager call. No machine-specific instance id
  is used.

### Dependencies (verified on the built binary)

The driver depends on no third-party component. Import table of `periph_vpad.sys`:

```
ntoskrnl.exe   — the Windows kernel
WDFLDR.SYS     — KMDF loader (Windows)
```

Everything else is in-box: `vhf.sys` (lower filter via `LowerFilters=vhf` in the INF) and
`hidclass.sys`. `vhfkm.lib` from the WDK is a link-time static library. Verify with
`llvm-objdump -p` or `dumpbin /DEPENDENTS` on `target
elease\periph_vpad.dll`.

## Hiding the physical pad (HidHide)

Without help, games see two DualSense controllers while emulating (physical + virtual) and get
double input. If [HidHide](https://github.com/nefarius/HidHide) (MIT, signed filter driver) is
installed, the `dualsense` driver uses its control device (`crates/hidhide`) when emulation
starts:

1. adds Periph's own image (`\Device\HarddiskVolumeN\...\periph.exe`) to the whitelist, unless
   HidHide is in inverse mode;
2. adds the physical pad's HID instance id to the blacklist;
3. turns hiding on if it was off.

It only extends the user's lists and later removes exactly what it added (HidHide's docs ask
third-party apps not to replace the lists). The changes are recorded in
`%LOCALAPPDATA%\Periph\hidhide-dualsense.txt` until undone; if Periph crashes, the next start
reverts them. The control device allows one handle at a time, so it is opened per operation.

## Periph driver JSON contract (for the UI)

Device id: `dualsense`. `Kind::Controller`.

Config (`read_config`, camelCase; the session starts with these values on every connect):
```json
{ "emulation": false, "forwardFeedback": true, "hidePhysical": true }
```

Status (`poll`; sent on visible changes, counters at most every 250 ms):
```json
{ "virtualActive": true, "batteryRaw": 42, "outputReports": 1234, "hidHide": "hiding" }
```
`hidHide` is `"missing"` (not installed: games see two pads), `"off"` or `"hiding"`.
`batteryRaw` is byte 53 of the input report (low nibble level, high nibble state).

Commands (`apply`, `{ "type": ..., "value": ... }`):
- `{ "type": "refresh" }`
- `{ "type": "emulation", "value": true }` — create/destroy the virtual device (and hide/unhide)
- `{ "type": "forwardFeedback", "value": false }` — don't forward OUT 0x02 to the physical pad
- `{ "type": "hidePhysical", "value": false }` — keep the physical pad visible while emulating

Disconnect codes (`Absent`): `notFound`, `emulatorRunning` (only an emulated pad is visible:
another emulator is hiding the physical one), `hidden` (the pad exists but cannot be opened
while HidHide is installed: usually another app left it blacklisted without whitelisting Periph). Error strings live in
`app/src/devices/dualsense/locales/` under `dualsense.errors.*`.
