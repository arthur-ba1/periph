#!/usr/bin/env python3
"""Condense a hidhook JSONL log: markers, writes, and meaningful reads.

Collapses busy polls (report 0x06 input reports starting with 02 00 00 ...) and
repeated identical status reports.

Usage: python tools/summarize.py captures/session.jsonl [--since-marker TEXT]
"""
import argparse
import json


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("log")
    ap.add_argument("--since-marker", help="start at the last marker containing this text")
    ap.add_argument("--status", action="store_true", help="also show changed 0x07 status reports")
    a = ap.parse_args()

    rows = [json.loads(l) for l in open(a.log, encoding="utf-8") if l.strip()]
    if a.since_marker:
        idx = [i for i, r in enumerate(rows) if r.get("ev") == "marker" and a.since_marker in r["text"]]
        rows = rows[idx[-1]:] if idx else rows

    last_status = None
    busy = 0
    t0 = rows[0]["t"] if rows else 0
    for r in rows:
        t = r["t"] - t0
        if r.get("ev") == "marker":
            print(f"\n===== {t:8.3f}  MARKER: {r['text']}")
            continue
        if r.get("ev") != "call":
            print(f"{t:8.3f}  {r.get('ev')} {r.get('path') or ''}")
            continue
        fn, data = r["fn"], (r.get("data") or "").upper()
        if fn == "hid_read_timeout" and data.startswith("07 20 02"):
            if a.status and data != last_status:
                print(f"{t:8.3f}  status  {data}")
            last_status = data
            continue
        if fn == "hid_get_input_report" and data.startswith("02 00 00 00 00"):
            busy += 1
            continue
        if busy:
            print(f"{'':8}  (busy x{busy})")
            busy = 0
        arrow = ">>" if fn in ("hid_write", "hid_send_feature_report", "hid_send_output_report") else "<<"
        print(f"{t:8.3f}  {arrow} {fn:<24} {data}")


if __name__ == "__main__":
    main()
