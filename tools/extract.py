#!/usr/bin/env python3
"""Extract HID configuration traffic from a USBPcap .pcapng capture.

Prints control transfers (SET_REPORT / GET_REPORT) and interrupt transfers that
carry data, skipping plain mouse movement on interface 0.

Usage:
    python tools/extract.py captures/04-dpi1-800.pcapng [--addr 5] [--all]
    python tools/extract.py A.pcapng B.pcapng --diff     # byte-level diff of payloads

Requires tshark (ships with Wireshark) on PATH or in the default install dir.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys

TSHARK_DEFAULT = r"C:\Program Files\Wireshark\tshark.exe"
REQ = {0x09: "SET_REPORT", 0x01: "GET_REPORT", 0x0A: "SET_IDLE", 0x06: "GET_DESCRIPTOR"}
RTYPE = {1: "IN", 2: "OUT", 3: "FEAT"}


def tshark() -> str:
    exe = shutil.which("tshark") or (TSHARK_DEFAULT if os.path.exists(TSHARK_DEFAULT) else None)
    if not exe:
        sys.exit("tshark not found; install Wireshark")
    return exe


def first(layer: dict, key: str):
    v = layer.get(key)
    return v[0] if isinstance(v, list) else v


def hexbytes(s: str | None) -> bytes:
    if not s:
        return b""
    return bytes.fromhex(s.replace(":", ""))


def packets(path: str, addr: int | None):
    out = subprocess.run(
        [tshark(), "-r", path, "-T", "json", "-x"], capture_output=True, text=True, check=True
    ).stdout
    for p in json.loads(out or "[]"):
        L = p["_source"]["layers"]
        usb = L.get("usb", {})
        dev = int(first(usb, "usb.device_address") or -1)
        if addr is not None and dev != addr:
            continue
        frame = int(first(L.get("frame", {}), "frame.number"))
        t = float(first(L.get("frame", {}), "frame.time_relative"))
        ttype = int(first(usb, "usb.transfer_type") or "0", 16)
        ep = int(first(usb, "usb.endpoint_address") or "0", 16)
        irp_in = first(usb, "usb.irp_info_tree") or {}
        direction = "IN " if (ep & 0x80) else "OUT"

        setup = L.get("Setup Data", {})
        data = hexbytes(first(L, "usb.capdata") or first(setup, "usb.data_fragment"))
        if not data:
            # some tshark versions put the payload here
            data = hexbytes(first(L.get("usbhid.data", {}), "usbhid.data")) if isinstance(L.get("usbhid.data"), dict) else b""

        desc = ""
        if setup:
            req = int(first(setup, "usb.setup.bRequest") or 0)
            wvalue = int(first(setup, "usb.setup.wValue") or 0)
            windex = int(first(setup, "usb.setup.wIndex") or 0)
            rtype, rid = (wvalue >> 8) & 0xFF, wvalue & 0xFF
            desc = f"{REQ.get(req, hex(req))} {RTYPE.get(rtype, rtype)} id=0x{rid:02X} if={windex}"
        elif ttype == 0x01:
            desc = "INTERRUPT"
        elif ttype == 0x02:
            desc = "CONTROL"
        yield dict(frame=frame, t=t, dev=dev, ep=ep, dir=direction, desc=desc, data=data)


def show(path: str, addr: int | None, show_all: bool):
    for p in packets(path, addr):
        if not p["data"]:
            continue
        # skip mouse movement: interrupt IN on endpoint 0x81 (interface 0)
        if not show_all and p["desc"] == "INTERRUPT" and p["ep"] == 0x81:
            continue
        print(f"{p['frame']:6} {p['t']:9.4f} dev={p['dev']} ep=0x{p['ep']:02X} {p['dir']} "
              f"{p['desc']:<32} {p['data'].hex(' ').upper()}")


def config_payloads(path: str, addr: int | None) -> list[bytes]:
    return [p["data"] for p in packets(path, addr)
            if p["data"] and not (p["desc"] == "INTERRUPT" and p["ep"] == 0x81)]


def diff(a: str, b: str, addr: int | None):
    pa, pb = config_payloads(a, addr), config_payloads(b, addr)
    print(f"{a}: {len(pa)} payloads   {b}: {len(pb)} payloads")
    for i in range(max(len(pa), len(pb))):
        x = pa[i] if i < len(pa) else b""
        y = pb[i] if i < len(pb) else b""
        if x == y:
            continue
        marks = "".join("^^ " if (j >= len(x) or j >= len(y) or x[j] != y[j]) else "   "
                        for j in range(max(len(x), len(y))))
        print(f"\n#{i}\nA: {x.hex(' ').upper()}\nB: {y.hex(' ').upper()}\n   {marks}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+")
    ap.add_argument("--addr", type=int, help="USB device address (from Wireshark)")
    ap.add_argument("--all", action="store_true", help="include mouse movement")
    ap.add_argument("--diff", action="store_true", help="diff payloads of two captures")
    a = ap.parse_args()
    if a.diff:
        if len(a.files) != 2:
            sys.exit("--diff needs exactly two files")
        diff(a.files[0], a.files[1], a.addr)
    else:
        for f in a.files:
            print(f"=== {f}")
            show(f, a.addr, a.all)


if __name__ == "__main__":
    main()
