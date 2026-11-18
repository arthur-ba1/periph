# VT2 Max assets (not in the repository)

The renders used by the VT2 Max module (`background_*.png`, `thumb_*.png`, the button
overlay `*.svg` files and `device.json`) belong to Shenzhen Rapoo Technology. They come from
the official A HUB installation and are **not distributed** with Periph: this folder is
git-ignored except for this note.

Without them the app works normally; the mouse render is replaced by a plain placeholder and
the color picker is hidden.

## Regenerating them from your own A HUB install

A HUB (tested with v1.0.19) keeps its images in Qt resources inside `A HUB.exe`.

```
python -I tools/carve_images.py "C:/Program Files/A HUB/A HUB.exe" %TEMP%/ahub-carve
python -I tools/rcc_extract.py "C:/Program Files/A HUB/A HUB.exe" %TEMP%/ahub-rcc VT2-MAX.json %TEMP%/ahub-carve/index.tsv
```

Then copy into this folder:

| from `%TEMP%/ahub-rcc/` | to here |
| --- | --- |
| `Image/Keys/VT2/background_<n>.png` | `background_<n>.png` |
| `Image/Keys/VT2/*.svg` | same names (`Mouse left.svg`, `DPI+.svg`, `DIY.svg`, ...) |
| `Image/Device/VT2_<n>.png` | `thumb_<n>.png` |
| `DeviceProfile/Device/VT2-MAX.json` | `device.json` |

`<n>` is the color variant key from `device.json` (`ProductColorType[].key`). Exact resource
paths can change between A HUB versions; list the extracted tree if a file is missing.

`model.glb` (optional 3D view, `?3d`) is generated locally with the ML scripts and is not
distributed either.

Use these files only on your own machine and never publish builds that contain them.
