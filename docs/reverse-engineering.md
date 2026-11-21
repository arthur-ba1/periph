# Reverse engineering a vendor configuration protocol

How the Rapoo VT2 Max support was built, written so it can be repeated for another device.
The result of this process is a protocol document (example: [protocol.md](protocol.md)) and a
pure protocol crate whose tests replay captured bytes (example: `crates/vt2-proto`).

Platform: Windows (where the vendor apps run). Tools: Rust, Python 3 with
`pip install frida-tools`, the official vendor software, and optionally Wireshark + USBPcap.

**Ground rule: observe, don't experiment.** Everything sent to the device later must be
something the official app was seen sending. Discovery tools in this repo only read.

## 1. Find the device's HID interfaces (read-only)

`periph-pack probe` (`periph-pack probe --vid <hex>`, `--listen <s>`) does the same for any vendor
and ships in the release kit; `vt2-probe` below is the repository's older tool.

```
cargo run -p vt2-probe -- --vid 24ae            # list interfaces + parsed report descriptors
cargo run -p vt2-probe -- --vid 24ae --listen 10  # also print input reports for 10 s
```

Look for collections on a vendor usage page (`0xFF00`–`0xFFFF`, marked `[VENDOR]`): that is
where configuration traffic goes. Note for each: PID (wired and wireless dongles usually have
different PIDs), interface number (`MI_xx`), usage page/usage, report IDs and their sizes, and
whether they are input, output or feature reports.

Wireless devices often expose the same interfaces through the dongle; the dongle may answer
even when the device is asleep (VT2: status byte `0x00` and an echo of the command).

## 2. Find how the vendor app talks to it

Start the vendor app, then list the process and the modules that export hidapi:

```
frida-ps | findstr /i "<app name>"
frida -n "A HUB.exe" -q -e "Process.enumerateModules().forEach(m => { if (m.findExportByName('hid_write')) console.log(m.name) })"
```

- A module exports `hid_write`/`hid_get_feature_report`: use it as `--module` below (A HUB:
  `RapooDevice.dll`). This gives the cleanest log: one line per hidapi call with the buffer.
- No module does: the app talks to Windows HID directly. `hidhook.py` then hooks `hid.dll`
  (`HidD_SetFeature`, `HidD_GetFeature`, `HidD_SetOutputReport`, `HidD_GetInputReport`) and
  `WriteFile`/`ReadFile` on HID handles. Asynchronous `ReadFile` data is not captured; if input
  reports are missing, fall back to USBPcap (section 6).

## 3. Capture, one change at a time

```
python tools/hidhook.py captures/session.jsonl --process "A HUB.exe" --module RapooDevice.dll
```

In another terminal, before each action in the vendor app, append a marker:

```
echo dpi stage 1 to 800 >> captures/marker.txt
```

Recipe (adapt to the device's features):

1. idle 15 s with the app open (background polling, status reports)
2. close and reopen the app (it reads the whole config: gives the read commands and memory map)
3. for every setting: change it once, marker, change it to a second value, marker, and a third
   value that reveals the encoding (e.g. DPI 800, 1600, 1650 shows units and step)
4. for enumerations (polling rate, modes): one marker per value
5. button remaps: one target per marker (another mouse button, a key, disabled, the default)
6. unplug/replug and sleep/wake, to see how errors look

Write down the exact before/after values the app showed for each marker.

## 4. Decode

```
python tools/summarize.py captures/session.jsonl --since-marker "dpi stage 1"
```

`summarize.py` drops repeated polls so only writes and meaningful reads remain. Work out, in
this order, and record each fact in the protocol doc with the marker it came from:

1. framing: report id, length, padding, command/opcode byte(s), length and address fields
2. response handshake: busy/ready status, how long the app retries, where data starts
3. read commands from the app startup (step 2): the memory map, block by block
4. each setting: which address changes and how the value is encoded (units, endianness, bit
   flags, lookup tables). Check that every captured value fits the formula
5. spontaneous input reports (battery, current DPI, connection type)
6. the write pattern: does the app read-modify-write? whole block or one byte?

Anything not fully explained stays "unknown" and read-only.

## 5. Implement, test, verify

1. **Protocol crate** (`crates/<id>-proto`): pure functions that build commands and parse
   responses, no I/O. Unit tests use the exact bytes from the capture as fixtures: building
   the "DPI 800" command must produce byte for byte what the app sent.
2. **Transport** (`crates/<id>-device` or a module in the driver): open the right interface,
   send, wait for ready, parse. Start read-only: read the whole config and compare it with what
   the vendor app shows.
3. **Writes**, one at a time: implement a command, change the setting from Periph, then open
   the vendor app and confirm it shows the same value. Close the vendor app first: two programs
   on the same HID channel interfere.
4. **Driver + module**: see [adding-a-device.md](adding-a-device.md).

## 6. When hooking is not possible: USBPcap

Install Wireshark with USBPcap, capture on the controller the device is attached to, and do
one change per capture file. [capture-plan.md](capture-plan.md) is the VT2 checklist used for
this route. Useful filter: `usb.idVendor == 0x24ae || usbhid`.

## 7. Vendor artwork

Renders make the UI much nicer but are copyrighted: extract them only for local use and never
commit them (see "Vendor artwork" in adding-a-device.md).

- Qt apps keep images in rcc resources inside the exe: `tools/carve_images.py` finds embedded
  PNG/WebP/JPEG; `tools/rcc_extract.py` rebuilds the resource tree with real file names.
- Electron apps: `npx asar extract resources/app.asar out/`.
- Others: look for a resources folder next to the exe, or carve the binary.

## 8. What not to publish

- Captures from your own device can contain its serial number and pairing data: check before
  committing them, or keep only the minimal excerpts the tests need.
- Never commit vendor binaries, DLLs or artwork.
