// Frida script: log the HID traffic of a vendor configuration app.
// Read-only observation of the official app; we never alter arguments or return values.
//
// Two layers, both optional:
// - hidapi exports (hid_write, hid_get_feature_report, ...) in CFG.module, if the app ships a
//   DLL that exports them (A HUB: RapooDevice.dll). Gives clean per-call buffers.
// - Windows hid.dll (HidD_SetFeature/GetFeature/SetOutputReport/GetInputReport) and WriteFile /
//   synchronous ReadFile on handles opened with a HID device path (one containing "hid#"). Works for apps that talk
//   to HID directly. Overlapped (async) ReadFile data is not captured.
// CFG is injected by hidhook.py: { module: "Some.dll" | null, winhid: "auto" | "on" | "off" }.
// "auto" hooks the Windows layer only when the module has no hidapi exports (otherwise every
// hidapi call would be logged twice, since hidapi itself calls WriteFile/HidD_*).

const CFG = __CFG__;
const MOD = CFG.module;
const devPath = {};
const modObj = MOD ? Process.findModuleByName(MOD) : null;
const HAS_HIDAPI = !!(modObj && modObj.findExportByName('hid_write'));
const WINHID = CFG.winhid === 'on' || (CFG.winhid === 'auto' && !HAS_HIDAPI);

function hex(ptr, len) {
  if (len <= 0) return '';
  const b = new Uint8Array(ptr.readByteArray(len));
  return Array.from(b, x => x.toString(16).padStart(2, '0')).join(' ');
}

function hook(name, cb) {
  if (!MOD) return;
  const m = Process.findModuleByName(MOD);
  const addr = m && m.findExportByName(name);
  if (!addr) { send({ ev: 'missing', name }); return; }
  Interceptor.attach(addr, cb);
}

function emit(fn, dev, data, len, extra) {
  send(Object.assign({ ev: 'call', fn, dev: dev.toString(), path: devPath[dev.toString()] || null,
                       len, data: data ? hex(data, len) : null }, extra || {}));
}

hook('hid_open_path', {
  onEnter(a) { this.p = a[0].readUtf8String(); },
  onLeave(r) { devPath[r.toString()] = this.p; send({ ev: 'open', dev: r.toString(), path: this.p }); }
});
hook('hid_close', { onEnter(a) { send({ ev: 'close', dev: a[0].toString(), path: devPath[a[0].toString()] || null }); } });

for (const fn of ['hid_write', 'hid_send_feature_report', 'hid_send_output_report']) {
  hook(fn, { onEnter(a) { emit(fn, a[0], a[1], a[2].toInt32()); } });
}
for (const fn of ['hid_read_timeout', 'hid_read', 'hid_get_feature_report', 'hid_get_input_report']) {
  hook(fn, {
    onEnter(a) { this.dev = a[0]; this.buf = a[1]; this.req = a[2].toInt32();
                 if (fn === 'hid_get_feature_report' || fn === 'hid_get_input_report')
                   this.reqData = hex(a[1], 1); },
    onLeave(r) { const n = r.toInt32(); if (n > 0) emit(fn, this.dev, this.buf, n, { req: this.req, reqId: this.reqData }); }
  });
}
// ---- Windows HID layer ----
function hookWin(mod, name, cb) {
  const addr = Module.findExportByName(mod, name);
  if (!addr) { send({ ev: 'missing', name: mod + '!' + name }); return; }
  Interceptor.attach(addr, cb);
}
if (WINHID) {
  try { Module.load('hid.dll'); } catch (e) {}
  const hidHandles = {};
  hookWin('kernel32.dll', 'CreateFileW', {
    onEnter(a) { this.p = a[0].isNull() ? '' : a[0].readUtf16String(); },
    onLeave(r) {
      if (this.p && this.p.toLowerCase().indexOf('hid#') >= 0 && r.toInt32() !== -1) {
        hidHandles[r.toString()] = true; devPath[r.toString()] = this.p;
        send({ ev: 'open', dev: r.toString(), path: this.p });
      }
    }
  });
  hookWin('kernel32.dll', 'CloseHandle', {
    onEnter(a) { const k = a[0].toString(); if (hidHandles[k]) { delete hidHandles[k]; send({ ev: 'close', dev: k, path: devPath[k] || null }); } }
  });
  hookWin('kernel32.dll', 'WriteFile', {
    onEnter(a) { if (hidHandles[a[0].toString()]) emit('WriteFile', a[0], a[1], a[2].toInt32()); }
  });
  hookWin('kernel32.dll', 'ReadFile', {
    onEnter(a) { this.dev = a[0]; this.buf = a[1]; this.nread = a[3]; this.on = !!hidHandles[a[0].toString()]; },
    onLeave(r) {
      if (!this.on || r.toInt32() === 0 || this.nread.isNull()) return; // async read: data arrives later
      const n = this.nread.readU32(); if (n > 0) emit('ReadFile', this.dev, this.buf, n);
    }
  });
  for (const fn of ['HidD_SetFeature', 'HidD_SetOutputReport']) {
    hookWin('hid.dll', fn, { onEnter(a) { emit(fn, a[0], a[1], a[2].toInt32()); } });
  }
  for (const fn of ['HidD_GetFeature', 'HidD_GetInputReport']) {
    hookWin('hid.dll', fn, {
      onEnter(a) { this.dev = a[0]; this.buf = a[1]; this.len = a[2].toInt32(); this.reqId = hex(a[1], 1); },
      onLeave(r) { if (r.toInt32()) emit(fn, this.dev, this.buf, this.len, { reqId: this.reqId }); }
    });
  }
}
send({ ev: 'ready', module: MOD, hidapi: HAS_HIDAPI, winhid: WINHID });
