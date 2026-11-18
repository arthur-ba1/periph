#!/usr/bin/env python3
"""Carve embedded PNG / WebP / JPEG images out of a binary (e.g. Qt rcc data in an exe).

Usage: python -I tools/carve_images.py "C:/Program Files/A HUB/A HUB.exe" OUT_DIR [--min 200]
Writes OUT_DIR/index.tsv with offset, format, width, height, size.
"""
import mmap
import os
import struct
import sys


def png_dims(b, i):
    # IHDR right after signature: len(4) 'IHDR' w h
    return struct.unpack(">II", b[i + 16:i + 24])


def webp_dims(b, i):
    fourcc = b[i + 12:i + 16]
    if fourcc == b"VP8X":
        w = 1 + int.from_bytes(b[i + 24:i + 27], "little")
        h = 1 + int.from_bytes(b[i + 27:i + 30], "little")
        return w, h
    if fourcc == b"VP8 ":
        w, h = struct.unpack("<HH", b[i + 26:i + 30])
        return w & 0x3FFF, h & 0x3FFF
    if fourcc == b"VP8L":
        v = int.from_bytes(b[i + 21:i + 25], "little")
        return (v & 0x3FFF) + 1, ((v >> 14) & 0x3FFF) + 1
    return 0, 0


def main(path, out, min_side):
    os.makedirs(out, exist_ok=True)
    rows = []
    with open(path, "rb") as f, mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as b:
        n = 0
        # PNG
        i = b.find(b"\x89PNG\r\n\x1a\n")
        while i != -1:
            end = b.find(b"IEND\xaeB`\x82", i)
            if end == -1:
                break
            end += 8
            w, h = png_dims(b, i)
            if max(w, h) >= min_side and w < 20000 and h < 20000:
                name = f"png_{i:010x}_{w}x{h}.png"
                open(os.path.join(out, name), "wb").write(b[i:end])
                rows.append((i, "png", w, h, end - i, name))
                n += 1
            i = b.find(b"\x89PNG\r\n\x1a\n", end)
        # WebP
        i = b.find(b"RIFF")
        while i != -1:
            if b[i + 8:i + 12] == b"WEBP":
                size = struct.unpack("<I", b[i + 4:i + 8])[0] + 8
                w, h = webp_dims(b, i)
                if max(w, h) >= min_side and size < 50_000_000:
                    name = f"webp_{i:010x}_{w}x{h}.webp"
                    open(os.path.join(out, name), "wb").write(b[i:i + size])
                    rows.append((i, "webp", w, h, size, name))
                i = b.find(b"RIFF", i + max(size, 12))
            else:
                i = b.find(b"RIFF", i + 4)
    with open(os.path.join(out, "index.tsv"), "w") as f:
        for r in sorted(rows, key=lambda r: -r[2] * r[3]):
            f.write("\t".join(map(str, r)) + "\n")
    print(f"{len(rows)} images >= {min_side}px written to {out}")


if __name__ == "__main__":
    a = sys.argv
    main(a[1], a[2], int(a[a.index("--min") + 1]) if "--min" in a else 200)
