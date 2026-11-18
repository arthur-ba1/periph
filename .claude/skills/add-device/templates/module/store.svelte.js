// <Brand Model> state: a view over the generic device registry ("mydevice" Rust driver).
import { deviceState, send as sendTo } from "$lib/device-state.svelte.js";

/** { connected, product, serial, reason, config, status, busy } */
export const dev = deviceState("mydevice");

/** `type` matches the Rust `Cmd` enum (camelCase). */
export const send = (type, value) => sendTo("mydevice", value === undefined ? { type } : { type, value });

/** Values the official software offers (the driver rejects anything else). */
export const POLLING = [125, 250, 500, 1000];

// ---- browser demo (?demo) ----
export const demo = {
  state: { product: "Brand Model", serial: "DEMO-0001", config: { polling: 1000 }, status: null },
  apply(d, { type, value }) {
    if (type === "polling") d.config.polling = value;
  },
};
