# <Brand Model> assets (not in the repository)

Renders from the official <vendor app> belong to <vendor>. They are not distributed with
Periph: this folder is git-ignored except for this note. Without them the module shows a
neutral placeholder.

## Regenerating them from your own <vendor app> install

<exact commands, e.g. tools/carve_images.py + tools/rcc_extract.py for Qt apps, or
`npx asar extract` for Electron apps>, then copy:

| from | to here |
| --- | --- |
| `<path inside the extracted resources>` | `<file name the module loads>` |

Use these files only on your own machine and never publish builds that contain them.
