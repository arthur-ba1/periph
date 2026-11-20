#!/usr/bin/env python3
"""Attach Frida to a vendor configuration app and log its HID traffic to a JSONL file.

Usage:
    python tools/hidhook.py captures/session.jsonl [--process "A HUB.exe"] [--module RapooDevice.dll] [--winhid auto|on|off]

--module: a DLL of the app that exports hidapi functions (hid_write, hid_get_feature_report...).
          Find candidates with: frida-ps, then list modules/exports (see docs/reverse-engineering.md).
          Pass --module "" if there is none.
--winhid: hook Windows hid.dll / WriteFile / ReadFile on HID handles. "auto" (default) does it
          only when --module has no hidapi exports, to avoid logging each call twice.

Each line: {"t": unix_time, "ev": "call"|"open"|..., "fn": ..., "path": ..., "data": "hex"}.
Write a marker line while it runs by appending text to captures/marker.txt
(each new line there is copied into the log as {"ev":"marker","text":...}).
"""
import argparse
import json
import os
import sys
import time

import frida

HERE = os.path.dirname(os.path.abspath(__file__))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--process", default="A HUB.exe")
    ap.add_argument("--module", default="RapooDevice.dll", help="DLL exporting hidapi functions, or '' for none")
    ap.add_argument("--winhid", choices=["auto", "on", "off"], default="auto")
    ap.add_argument("--marker", default=os.path.join(HERE, "..", "captures", "marker.txt"))
    a = ap.parse_args()

    os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
    out = open(a.out, "a", encoding="utf-8", buffering=1)

    def log(obj):
        obj["t"] = round(time.time(), 4)
        out.write(json.dumps(obj) + "\n")

    def on_message(msg, _data):
        if msg["type"] == "send":
            log(msg["payload"])
        else:
            log({"ev": "error", "msg": msg})

    # A PID picks one process when several share a name (e.g. a launcher and the real app).
    session = frida.attach(int(a.process) if a.process.isdigit() else a.process)
    cfg = json.dumps({"module": a.module or None, "winhid": a.winhid})
    source = open(os.path.join(HERE, "hidhook.js"), encoding="utf-8").read().replace("__CFG__", cfg)
    script = session.create_script(source)
    script.on("message", on_message)
    script.load()
    print(f"hooked {a.process}, logging to {a.out}", flush=True)

    open(a.marker, "a").close()
    seen = os.path.getsize(a.marker)
    try:
        while True:
            time.sleep(0.2)
            size = os.path.getsize(a.marker)
            if size > seen:
                with open(a.marker, encoding="utf-8") as f:
                    f.seek(seen)
                    for line in f.read().splitlines():
                        if line.strip():
                            log({"ev": "marker", "text": line.strip()})
                            print("marker:", line.strip(), flush=True)
                seen = size
    except KeyboardInterrupt:
        pass
    finally:
        session.detach()


if __name__ == "__main__":
    sys.exit(main())
