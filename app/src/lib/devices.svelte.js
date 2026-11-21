// Device registry: discovers the modules in src/devices/<id>/ and wires them to the backend.
//
// Every folder in src/devices/<id>/ with an index.js is a device module (folders starting with
// "_" are ignored). The Rust driver with the same id feeds it through the "device" event:
//   { device: "<id>", type: "connected" | "disconnected" | "config" | "status" | "error" | "done", ... }
// and receives commands through `send(id, cmd)` (shape defined by the driver).
//
// Live state lives in device-state.svelte.js so device modules can import it without a cycle.
// See docs/adding-a-device.md for what a module exports.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "svelte-sonner";
import { devices, deviceState, setDemoHandler, DEMO } from "./device-state.svelte.js";
import { t } from "./i18n.svelte.js";

export { devices, deviceState, send, DEMO } from "./device-state.svelte.js";

const found = import.meta.glob("../devices/*/index.js", { eager: true, import: "default" });

/** Device modules, sorted by `order`. */
export const MODULES = Object.entries(found)
  .filter(([path]) => !path.split("/").at(-2).startsWith("_"))
  .map(([, m]) => m)
  .sort((a, b) => (a.order ?? 99) - (b.order ?? 99));
MODULES.forEach((m) => {
  deviceState(m.id);
  if (m.demo?.apply) setDemoHandler(m.id, m.demo.apply);
});

export const moduleFor = (id) => MODULES.find((m) => m.id === id);

/** Add a module built at runtime (a device package, see lib/packages.js). Call before the app
 *  mounts: MODULES is not reactive. A module that `replaces` a built-in one removes it. */
export function addModule(m) {
  const drop = MODULES.findIndex((x) => x.id === m.replaces || x.id === m.id);
  if (drop >= 0) MODULES.splice(drop, 1);
  MODULES.push(m);
  MODULES.sort((a, b) => (a.order ?? 99) - (b.order ?? 99));
  deviceState(m.id);
  if (m.demo?.apply) setDemoHandler(m.id, m.demo.apply);
}

/** Name to show for a device (modules may derive it from the config, e.g. the VIA definition). */
export const displayName = (m) => m.displayName?.(devices[m.id]) ?? m.name;
// (module `name` may be a getter calling t(), so it follows the UI language)

function onEvent(id, e) {
  const d = devices[id], m = moduleFor(id);
  if (!d) return;
  switch (e.type) {
    case "connected":
      Object.assign(d, { connected: true, product: e.product, serial: e.serial, reason: null, code: null });
      break;
    case "disconnected":
      // only announce losing a device we actually had loaded (retries while absent stay quiet)
      if (d.config) toast.warning(t("app.toastDisconnected", { name: displayName(m) }));
      Object.assign(d, { connected: false, config: null, status: null, reason: e.reason, code: e.code ?? null, busy: false });
      break;
    case "config": {
      const first = !d.config;
      d.config = e.config;
      d.connected = true; // only a connected device reports a config
      if (first) toast.success(t("app.toastConnected", { name: displayName(m) }));
      break;
    }
    case "status":
      d.status = e.status;
      d.connected = true;
      break;
    case "error":
      toast.error(e.message);
      break;
    case "done":
      d.busy = false;
      break;
  }
}

export async function initDevices() {
  if (DEMO) {
    for (const m of MODULES) if (m.demo) Object.assign(devices[m.id], { connected: true, driver: true, reason: null, ...structuredClone(m.demo.state) });
    return;
  }
  // Subscribe before reading the snapshots: an event sent in between (typically "connected"
  // right at startup) would otherwise be lost. Events that arrive before the snapshots are
  // applied are replayed on top of them.
  let pending = [];
  const unlisten = await listen("device", ({ payload }) => {
    if (pending) pending.push(payload);
    else onEvent(payload.device, payload);
  });
  try {
    for (const entry of await invoke("devices")) {
      if (!devices[entry.id]) continue; // driver without a frontend module
      Object.assign(devices[entry.id], entry.snapshot, { driver: true });
    }
  } catch {
    unlisten();
    for (const d of Object.values(devices)) d.reason = t("app.noBackend");
    return;
  }
  const queued = pending;
  pending = null;
  for (const payload of queued) onEvent(payload.device, payload);
}
