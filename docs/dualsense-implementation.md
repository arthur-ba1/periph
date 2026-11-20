# DualSense virtual driver — implementation reference

Audience: an engineer (human or LLM) who must modify, extend or debug this code. It describes
what each component does, the exact contracts between them, the invariants that must hold, and
the known gaps. For wire-format details and the UI contract see `dualsense-protocol.md`; this
file is about the engineered system.

---

## 1. Components and data flow

```
┌──────────────────── user mode ─────────────────────┐   ┌──── kernel ─────┐
│ periph-app  drivers/dualsense.rs                   │   │ periph-vpad.sys │
│   ├ worker thread: poll() every 4 ms (output)      │   │  device.rs      │
│   └ forwarder thread: blocking HID read (input)    │   │   ├ ioctl.rs    │
│        ▲ hidapi            │ dualsense-vpad        │   │   └ vhf.rs      │
│  physical pad        ┌─────▼──────────┐ DeviceIoControl  │  output FIFO +  │
│  (054C:0CE6)         │ \\.\PeriphVpad │◀────────────────▶│  feature cache  │
│        ▲             └────────────────┘                  └───────┬─────────┘
│        └ hidden from other apps by HidHide (crates/hidhide)      │ VHF
└────────────────────────────────────────────────────┘    vhf.sys ─┴─▶ games see one DualSense
```

| Layer | Owns | Never does |
|---|---|---|
| `dualsense-proto` | byte layout of input/output/feature reports | any I/O |
| `dualsense-vpad` | `CreateFile`/`DeviceIoControl` to the control device | HID traffic, policy |
| `hidhide` | HidHide control device: extend/restore its lists, crash ledger | anything DualSense-specific |
| `periph-vpad` (kernel) | virtual HID device lifetime, VHF callbacks, output FIFO, feature cache | parsing policy, remapping |
| `drivers/dualsense.rs` | physical pad I/O, forwarding, watchdog, hiding policy, UI config | touching VHF or HID descriptors |

---

## 2. Kernel driver

The kernel driver (`periph-vpad`: lifecycle, IOCTL contract, VHF device, concurrency, feature
cache, build) lives in its own repository, with its implementation notes in
`docs/implementation.md` there. This repository only holds its user-mode client
(`crates/dualsense-vpad`, below).

---

## 3. User-mode channel (`crates/dualsense-vpad`)

`trait Channel` lets the Periph driver be tested without a kernel:

```rust
fn submit_input(&mut self, &InputReport) -> Result<()>;
fn read_output(&mut self) -> Result<Option<OutputReport>>;   // None = queue empty
fn set_feature_response(&mut self, &FeatureResponse) -> Result<()>;
fn is_active(&mut self) -> Result<bool>;
fn start(&mut self) -> Result<()>;
fn destroy(&mut self) -> Result<()>;
```

`WindowsChannel` wraps `CreateFileW("\\.\PeriphVpad")` and `DeviceIoControl`. `read_output`
returns `Ok(None)` for a 0-byte success, `Ok(Some)` for 48 bytes, and `Err` for any failure
(driver gone, device removed), which ends the session. `MockChannel` implements the same trait
in memory.

---

## 4. HidHide client (`crates/hidhide`)

See `dualsense-protocol.md` § "Hiding the physical pad" for the behaviour. Implementation notes:

- IOCTL codes and buffer formats come from the HidHide sources (`Shared/HidHideIoctlContract.h`,
  `HidHideCLI/src/FilterDriverProxy.cpp`): device type 32769, functions 2048–2054,
  `METHOD_BUFFERED`, `FILE_READ_DATA`; lists are UTF-16 MULTI_SZ, read with a size probe first;
  ACTIVE/WLINVERSE are one byte.
- The control device allows one handle at a time: `open()` retries for ~0.5 s, every operation
  opens and closes it.
- `hide_with` / `unhide_with` work on a `Control` trait (real device or `MockControl`) and only
  touch the entries they added; the whitelist is written before the blacklist so Periph never
  loses access to the pad.
- Whitelist entries are NT image paths: `nt_image_path` maps
  `GetVolumePathNameW` → `GetVolumeNameForVolumeMountPointW` → `QueryDosDeviceW`.
- The blacklist entry is the pad's HID instance id as the configuration manager spells it
  (`CM_Get_Device_IDW`), not the uppercased string derived from the HID path.

---

## 5. Periph driver (`crates/periph-app/src/drivers/dualsense.rs`)

### 5.1 Opening the physical pad

`PhysicalPad::open()` enumerates hidapi for `054C:0CE6`, usage page 0x01 / usage 0x05, and
rejects emulated devices with `is_virtual_path` (fast path: `periph`/`vhf` in the path;
otherwise walk PnP parents: `PCI\` ancestor = real, `ROOT\` ancestor = virtual). Errors are
`Absent` with codes `emulatorRunning` (only emulated pads visible) or `notFound`.

The pad may be on USB or Bluetooth (`Transport`, chosen from the HID path; USB is preferred when
both are present). Only USB-shaped paths go through `is_virtual_path`. Everything above the
HID handle works on USB reports: `Transport` turns Bluetooth input 0x31 into a USB 0x01 report,
USB output 0x02 into Bluetooth 0x31 with its CRC, and Bluetooth feature reports into their USB
form, so the virtual device, effects and watchdog don't depend on the transport. Opening a
Bluetooth pad reads feature 0x05 once to enable full input reports. Framing:
`docs/dualsense-protocol.md` ("Bluetooth").
`Driver::open` first calls `hidhide::recover` to undo hiding left by a crashed run.

### 5.2 Threads and loops

- **Forwarder thread** (`dualsense-input`, exists only while emulating): its own HID handle,
  `read_timeout(100 ms)`, and `submit_input` for every report the moment it arrives. Input
  latency therefore does not depend on the worker's poll timer (Windows timers tick at ~15 ms
  by default). It stops when `stop` is set or on error (stored in `failure`, which makes the
  next `poll()` fail and end the session).
- **Worker `poll()`**, every 4 ms while emulating (100 ms otherwise):
  1. drain the status handle (battery from byte 53; an error means the pad is gone);
  2. drain the kernel output FIFO; for each report, in order: drop what `feedback` keeps from
     the game (`Feedback::filter`), write Periph's kept sections over it, and send it to the
     physical pad unless nothing is left flagged;
  3. feed each (filtered) report to `RumbleWatchdog`; on expiry send `OutputReport::rumble_stop()`;
  4. emit status through `StatusThrottle` (visible changes at once, counters at most every
     250 ms; `periph-core` emits an event for every `Some`).

The channel `Mutex` is shared by both threads; every critical section is one short IOCTL.

### 5.3 Teardown

`Emulation` (channel + forwarder) is dropped to stop: set `stop`, join the thread, `destroy()`.
`DualSenseSession::drop` calls `stop_emulation()`, which drops the emulation first, then unhides
the physical pad (so games briefly see no pad rather than two), then stops leftover rumble.
The session drops on unplug, read error, or app exit; the kernel independently removes the pad
when the channel handle closes, covering crashes.

### 5.4 Commands

`refresh`, `emulation(on)`, `feedback({ lights, haptics, triggers })` (turning haptics off also
stops running motors), `hidePhysical(on)` (applies at once while emulating), plus the effect
sections. Everything but `emulation` is saved; emulation always starts off.

---

## 6. Feature behaviour

### 6.1 Buttons, sticks, triggers, touchpad

Pass-through: the virtual device reports exactly what the physical one reports (whole 64-byte
frame, so gyro, touchpad and timestamps pass untouched). Remapping belongs in the forwarder
before `submit_input`, using `InputReport::set_sticks` / `set_buttons`.

### 6.2 Rumble

Path: game → `evt_write_report` → FIFO → `poll()` → `write_output`. Byte 3 = right (strong)
motor, byte 4 = left (weak); they apply only when byte 1 bit 0 or 1 is set.

Watchdog (same 5 s policy as the vendor app): armed by a report that sets the rumble bits with a
non-zero motor, cancelled by one that sets them with both motors at 0, unaffected by reports
without the rumble bits. On expiry the driver sends a rumble-only stop report, never a copy of
an old report, so lightbar, LEDs and triggers are not reverted. If a game holds a long constant
rumble and the pad stops early, the window is what to raise.

### 6.3 Lightbar, player LEDs, mic LED, adaptive triggers

Same path (bytes 44–47, 9, 43, 39; trigger blocks at 11 and 22). The user picks a source per
category in the Emulation page (`config.feedback`): lights and triggers go to the game or to
Periph, the game's vibration is forwarded or dropped. A category kept for Periph has its valid
bits cleared in every game report (`OutputReport::drop_lights` / `drop_triggers` /
`drop_rumble`) and Periph's sections are written instead; a category left to the game passes
through, and Periph fills only the sections the game never set. The virtual device cannot
emulate trigger resistance itself: forwarding the game's trigger block is what makes the
effect reach the real pad.

### 6.4 Feature reports

`Emulation::start` reads the allowlisted reports from the physical pad (GET_FEATURE only),
uploads them, and only then sends `START`, so no client can enumerate the pad with empty slots.
Never fabricate these reports, and never commit captures (the MAC report is unit-specific).

### 6.5 Audio and haptics

Not supported, by design: on a real DualSense, haptics are a **USB audio** stream (4-channel
48 kHz, two channels for haptics), and VHF exposes HID only. Possible later, without kernel
changes: audio→rumble in user mode (WASAPI loopback → motor bytes), and haptics written
directly to the physical pad's audio endpoint (bypassing the virtual device). Receiving a
game's haptic stream would need a full virtual USB device with audio interfaces: a separate
project.

---

## 7. Invariants checklist (use when reviewing a change)

1. Input reports reaching `VhfReadReportSubmit` are exactly 64 bytes, id 0x01.
2. Output reports reaching the physical pad are exactly 48 bytes, id 0x02 (over Bluetooth:
   the same body as 78-byte 0x31 with a valid CRC), in arrival order.
3. No VHF create/start/delete and no allocation under the spin lock.
4. Every IOCTL path completes its request exactly once; an empty queue is a 0-byte success.
5. Feature responses are only taken from the allowlist and from the physical unit.
6. The virtual pad never outlives its owner handle, the session, or the device.
7. HidHide changes are only additions Periph made, recorded in the ledger, and undone on stop.
8. User-visible errors go through `t()` (`app/src/devices/dualsense/locales/`).
9. `cargo test --workspace` passes; the kernel crate builds with zero warnings.
10. Nothing writes firmware, calibration or pairing commands to hardware.

---

## 8. Known gaps / next steps

- The kernel driver is test-signed only; distribution needs Microsoft signing (EV certificate
  + attestation). Its own known gaps (one virtual pad per machine, default PnP security on the
  control device) are listed in the `periph-vpad` repository.
- Emulation is off after every reconnect (effects are persisted, emulation is not).
- Haptics over Bluetooth (reports 0x32/0x36/0x39) are not decoded.
