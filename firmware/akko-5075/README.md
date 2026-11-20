# Word lighting for the Akko 5075S VIA

This QMK keymap targets the wired Akko 5075S VIA with ANSI layout, USB ID `FFFE:000B`, and a WB32FQ95 MCU. It uses the keyboard and default keymap from [the source linked by Akko](https://en.akkogear.com/product/5075s-via/) at revision [`a94b7e4`](https://github.com/jonylee1986/qmk_firmware_master/commit/a94b7e4c948a9ffe265b073e69292fc0325c0f39).

## Build

In a [QMK build environment](https://docs.qmk.fm/newbs_getting_started), run from the Periph repository root:

```sh
git clone --branch hfd_5075 https://github.com/jonylee1986/qmk_firmware_master.git .qmk-build
git -C .qmk-build checkout a94b7e4c948a9ffe265b073e69292fc0325c0f39
git -C .qmk-build submodule update --init --depth 1 lib/chibios lib/chibios-contrib lib/printf
touch .qmk-build/keyboards/akko/5075/rules.mk
cp .qmk-build/keyboards/akko/5075/keyboard.json .qmk-build/keyboards/akko/5075/info.json
mkdir -p .qmk-build/keyboards/akko/5075/keymaps/periph_word
cp firmware/akko-5075/keymap.c firmware/akko-5075/rgb_matrix_user.inc firmware/akko-5075/rules.mk .qmk-build/keyboards/akko/5075/keymaps/periph_word/
cd .qmk-build
make SKIP_GIT=yes akko/5075:periph_word
```

The Akko branch uses a newer filename for its hardware definition than its QMK build scripts recognize. The empty board-level `rules.mk` and the copy named `info.json` make the board discoverable for that build. The resulting file is `akko_5075_periph_word.bin`. Check the label and hardware revision before loading a firmware image. Back up the VIA keymap and keep the [Akko recovery firmware for the matching model and layout](https://en.akkogear.com/faq/how-to-recover-a-5075s-via-keyboard-after-flashing-other-wrong-firmware/) available.

## Use on the keyboard

1. Press **Fn + Backspace**, then release **Fn**.
2. Type up to 32 letters or digits. **Backspace** removes the last character and **Esc** cancels.
3. Press **Enter** to play the new sequence. **Fn + knob press** advances to another RGB mode.

The keyboard lights the physical positions you typed, using the current RGB matrix color and brightness. The word stays in RAM until power is removed. The initial word is `ARTHUR`. VIA keymap and encoder support are enabled in this build. The word editor in Periph remains a screen preview.
