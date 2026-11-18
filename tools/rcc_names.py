#!/usr/bin/env python3
"""List Qt rcc resource names (UTF-16BE, length-prefixed) found in a binary that match a pattern.

rcc name entries are: u16 length, u32 hash, then `length` UTF-16BE code units.
Usage: python -I tools/rcc_names.py BINARY PATTERN
"""
import mmap
import re
import sys


def main(path, pattern):
    rx = re.compile(pattern, re.I)
    seen = set()
    with open(path, "rb") as f, mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as b:
        # UTF-16BE ASCII-ish runs of >= 3 chars
        for m in re.finditer(rb"(?:\x00[\x20-\x7e]){3,80}", b):
            s = m.group().decode("utf-16-be")
            start = m.start()
            # verify rcc header: 2-byte length right before 4-byte hash
            if start >= 6:
                ln = int.from_bytes(b[start - 6:start - 4], "big")
                if ln != len(s):
                    continue
            if rx.search(s) and s not in seen:
                seen.add(s)
                print(f"{start:#012x}  {s}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
