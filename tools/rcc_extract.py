#!/usr/bin/env python3
"""Extract a Qt5 rcc resource tree (names + tree + data) embedded in a binary.

Locates the names table by walking back from a known entry, finds the tree by
matching name offsets, infers the data base from carved PNG offsets, then
writes every file under OUT_DIR with its real path.

Usage: python -I tools/rcc_extract.py BINARY OUT_DIR ANCHOR_NAME CARVED_INDEX_TSV
  ANCHOR_NAME: a resource file name known to exist (e.g. "VT2-MAX.json")
"""
import collections
import mmap
import os
import re
import struct
import sys
import zlib

NODE = 22  # rcc format v2/v3: name u32, flags u16, (count u32, child u32 | country u16, lang u16, data u32), mtime u64


def be16(b, o): return struct.unpack_from(">H", b, o)[0]
def be32(b, o): return struct.unpack_from(">I", b, o)[0]


def entry_ok(b, q, end):
    """Is there a names entry at q that ends exactly at `end`?"""
    if q < 0:
        return False
    n = be16(b, q)
    if n == 0 or q + 6 + 2 * n != end:
        return False
    s = b[q + 6:end]
    return all(s[i] == 0 and 0x20 <= s[i + 1] < 0x7f for i in range(0, len(s), 2)) or _utf16_ok(s)


def _utf16_ok(s):
    try:
        t = s.decode("utf-16-be")
        return all(c.isprintable() for c in t)
    except UnicodeDecodeError:
        return False


def find_names_base(b, anchor_pos):
    p = anchor_pos  # start of a valid entry
    while True:
        found = None
        for n in range(1, 256):
            q = p - 6 - 2 * n
            if entry_ok(b, q, p):
                found = q
                break
        if found is None:
            return p
        p = found


def name_at(b, base, off):
    p = base + off
    n = be16(b, p)
    return b[p + 6:p + 6 + 2 * n].decode("utf-16-be")


def main(path, out, anchor, carved_tsv):
    f = open(path, "rb")
    b = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
    utf = anchor.encode("utf-16-be")
    hits = [m.start() - 6 for m in re.finditer(re.escape(utf), b) if be16(b, m.start() - 6) == len(anchor)]
    if not hits:
        sys.exit("anchor not found")
    a = hits[0]
    N = find_names_base(b, a)
    print(f"names base {N:#x} (anchor at {a:#x})")

    # find tree node pointing at the anchor name (file node: flags 0/1)
    key = struct.pack(">I", a - N)
    cands = [m.start() for m in re.finditer(re.escape(key), b) if be16(b, m.start() + 4) in (0, 1, 4)]
    print(f"{len(cands)} candidate nodes for anchor")
    # root: name 0, flags 2 (dir); choose root T such that (cand - T) % NODE == 0 and tree parses
    best = None
    for c in cands:
        for k in range(0, 4000):
            T = c - k * NODE
            if T < 0:
                break
            if be32(b, T) == 0 and be16(b, T + 4) == 2 and be32(b, T + 10) == 1:
                best = (T, c)
                break
        if best:
            break
    if not best:
        sys.exit("tree root not found")
    T, _ = best
    print(f"tree base {T:#x}")

    def node(i):
        o = T + i * NODE
        name_off, flags = be32(b, o), be16(b, o + 4)
        if flags & 2:
            return name_off, flags, be32(b, o + 6), be32(b, o + 10)
        return name_off, flags, None, be32(b, o + 10)

    files = []  # (path, flags, data_off)

    def walk(i, prefix):
        name_off, flags, count, x = node(i)
        name = "" if i == 0 else name_at(b, N, name_off)
        p = f"{prefix}/{name}" if name else prefix
        if flags & 2:
            for j in range(count):
                walk(x + j, p)
        else:
            files.append((p, flags, x))

    walk(0, "")
    print(f"{len(files)} files in tree")

    # infer data base D from carved PNG offsets
    pngs = set(int(l.split("\t")[0]) for l in open(carved_tsv) if l.strip() and l.split("\t")[1] == "png")
    votes = collections.Counter()
    png_nodes = [x for x in files if x[0].lower().endswith(".png") and not (x[1] & 1)]
    for _, _, do in png_nodes[:200]:
        for po in pngs:
            votes[po - 4 - do] += 1
    D, n = votes.most_common(1)[0]
    print(f"data base {D:#x} ({n} votes)")

    os.makedirs(out, exist_ok=True)
    ok = 0
    for p, flags, do in files:
        o = D + do
        size = be32(b, o)
        data = b[o + 4:o + 4 + size]
        if flags & 1:  # zlib: 4-byte BE uncompressed size + zlib stream
            try:
                data = zlib.decompress(data[4:])
            except zlib.error:
                continue
        dst = os.path.join(out, p.lstrip("/").replace("/", os.sep))
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        open(dst, "wb").write(data)
        ok += 1
    print(f"extracted {ok} files to {out}")


if __name__ == "__main__":
    main(*sys.argv[1:5])
