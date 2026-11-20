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
// Frida 17 removed the static Module.findExportByName; look the module up first.
function winExport(mod, name) {
  const m = Process.findModuleByName(mod);
  return m ? m.findExportByName(name) : null;
}
function hookWin(mod, name, cb) {
  const addr = winExport(mod, name);
  if (!addr) { send({ ev: 'missing', name: mod + '!' + name }); return; }
  Interceptor.attach(addr, cb);
}
if (WINHID) {
  try { Module.load('hid.dll'); } catch (e) {}
  // handle -> true (HID) / false (not HID). Handles opened before we attached are classified on
  // first WriteFile with HidD_GetAttributes, only for device-type handles: disk files and pipes
  // are skipped so we never issue an IOCTL on a handle that could block the app.
  const hidHandles = {};
  const getFileType = new NativeFunction(winExport('KERNELBASE.dll', 'GetFileType'), 'uint32', ['pointer']);
  const getAttributes = new NativeFunction(winExport('hid.dll', 'HidD_GetAttributes'), 'int', ['pointer', 'pointer']);
  function isHid(h) {
    const k = h.toString();
    if (k in hidHandles) return hidHandles[k];
    let hid = false;
    const type = getFileType(h);
    if (type !== 1 && type !== 3) { // not FILE_TYPE_DISK, not FILE_TYPE_PIPE
      const attrs = Memory.alloc(12);
      attrs.writeU32(12);
      if (getAttributes(h, attrs)) {
        hid = true;
        const id = 'vid_' + attrs.add(4).readU16().toString(16).padStart(4, '0') + '&pid_' + attrs.add(6).readU16().toString(16).padStart(4, '0');
        devPath[k] = devPath[k] || id;
        send({ ev: 'open', dev: k, path: devPath[k], preexisting: true });
      }
    }
    hidHandles[k] = hid;
    return hid;
  }
  hookWin('KERNELBASE.dll', 'CreateFileW', {
    onEnter(a) { this.p = a[0].isNull() ? '' : a[0].readUtf16String(); },
    onLeave(r) {
      if (this.p && this.p.toLowerCase().indexOf('hid#') >= 0 && r.toInt32() !== -1) {
        hidHandles[r.toString()] = true; devPath[r.toString()] = this.p;
        send({ ev: 'open', dev: r.toString(), path: this.p });
      }
    }
  });
  hookWin('KERNELBASE.dll', 'CloseHandle', {
    onEnter(a) {
      const k = a[0].toString();
      if (hidHandles[k]) send({ ev: 'close', dev: k, path: devPath[k] || null });
      delete hidHandles[k]; // handle values are reused: classify again next time
    }
  });
  hookWin('KERNELBASE.dll', 'WriteFile', {
    onEnter(a) { if (isHid(a[0])) emit('WriteFile', a[0], a[1], a[2].toInt32()); }
  });
  hookWin('KERNELBASE.dll', 'ReadFile', {
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
