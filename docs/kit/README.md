# Periph

Configure your mouse, keyboard or other peripheral without the manufacturer's software.
**English** · [Português](LEIAME.md)

## Use it

Run `Periph.exe`. Close the manufacturer's software first: both can't use the device at the
same time. Closing the window keeps Periph in the tray.

## Add your own device

Periph supports a device through a **device package**: a folder of JSON files describing how
the device is configured. Packages live in `%APPDATA%\Periph\devices\` and are loaded when
Periph starts; nothing is compiled.

The easiest way to write one is with an AI coding agent and the `add-device` skill in this kit:

1. Install Python and, for capturing the official app, `pip install frida-tools`.
2. Claude Code: copy the `add-device` folder into `%USERPROFILE%\.claude\skills\`. Other
   agents: point them at `add-device/SKILL.md`.
3. Open the agent in this folder and ask it to add your device ("add support for my
   <brand model> to Periph"). It will identify the device, ask you to use the official app
   while it captures the traffic, write the package, check it and help you test it.

`periph-pack.exe` is the toolbox the skill uses (all read-only on the device):

```
periph-pack probe                       list HID devices
periph-pack probe --vid 046D            interfaces of one device
periph-pack new <folder> --vid ...      start a package
periph-pack check <folder>              validate a package and replay its captured examples
periph-pack dir                         where Periph loads packages from
```

The format is described in `add-device/docs/device-packages.md`, with two complete examples
in `add-device/examples/`.

## Safety

Packages can only send commands that were captured from the official software, with values
the official software allows; the engine enforces it. Packages never update firmware, never
reset or calibrate the device, and can't reach anything but the device they describe.

Source code and issues: https://github.com/arthur-ba1/periph (MIT license).
