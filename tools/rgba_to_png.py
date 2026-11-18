#!/usr/bin/env python3
"""Convert raw 32x32 RGBA dumps (from the tray icon test) into one preview PNG.

Usage: python -I tools/rgba_to_png.py target/tray-preview out.png [--scale 4]
Each icon is drawn on a dark and a light tile so both taskbar themes can be judged.
"""
import os
import struct
import sys
import zlib

N = 32


def main(src, out, scale):
    files = sorted(f for f in os.listdir(src) if f.endswith(".rgba"))
    icons = [open(os.path.join(src, f), "rb").read() for f in files]
    tile = N * scale + 16
    W, H = tile * len(icons), tile * 2
    bgs = [(32, 32, 32), (238, 238, 238)]
    rows = []
    for y in range(H):
        row = bytearray([0])
        band, ty = divmod(y, tile)
        for x in range(W):
            idx, tx = divmod(x, tile)
            r, g, b = bgs[band]
            ix, iy = (tx - 8) // scale, (ty - 8) // scale
            if 0 <= ix < N and 0 <= iy < N:
                o = (iy * N + ix) * 4
                pr, pg, pb, pa = icons[idx][o:o + 4]
                a = pa / 255
                r, g, b = (int(pr * a + r * (1 - a)), int(pg * a + g * (1 - a)), int(pb * a + b * (1 - a)))
            row += bytes((r, g, b))
        rows.append(bytes(row))

    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b"")
    open(out, "wb").write(png)
    print("order:", ", ".join(f[:-5] for f in files))


if __name__ == "__main__":
    a = sys.argv
    main(a[1], a[2], int(a[a.index("--scale") + 1]) if "--scale" in a else 4)
