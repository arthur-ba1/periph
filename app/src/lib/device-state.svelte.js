// Live state per device, shared by the registry (devices.svelte.js) and the device modules.
import { invoke } from "@tauri-apps/api/core";
import { toast } from "svelte-sonner";

// Browser-only demo mode (`?demo`): every module provides a fake connected state.
export const DEMO = typeof window !== "undefined" && !window.__TAURI_INTERNALS__ && location.search.includes("demo");

const blank = () => ({ connected: false, product: null, serial: null, reason: null, config: null, status: null, busy: false, driver: false });

/** id -> { connected, product, serial, reason, config, status, busy, driver } */
export const devices = $state({});

/** The (reactive) state object of one device, created on first use. */
export function deviceState(id) {
  if (!devices[id]) devices[id] = blank();
  return devices[id];
}

const demoHandlers = {};
/** Registered by the registry from each module's `demo.apply`. */
export const setDemoHandler = (id, fn) => (demoHandlers[id] = fn);

/** Send a driver-specific command, e.g. send("vt2", { type: "polling", value: 1000 }). */
export function send(id, cmd) {
  const d = deviceState(id);
  d.busy = true;
  if (DEMO) {
    demoHandlers[id]?.(d, cmd);
    setTimeout(() => (d.busy = false), 200);
    return Promise.resolve();
  }
  return invoke("device_send", { device: id, cmd }).catch((e) => {
    d.busy = false;
    toast.error(String(e));
  });
}
