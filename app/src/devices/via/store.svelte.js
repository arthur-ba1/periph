// VIA keyboard state: a view over the generic device registry ("via" Rust driver).
import { deviceState, send } from "$lib/device-state.svelte.js";
import { definitionFor } from "./definitions.js";

const d = deviceState("via");
const cfg = () => d.config ?? {};

/** Getters over the live state. Pages may update nested arrays optimistically, then sendKbd(). */
export const kbd = {
  get connected() { return d.connected && !!d.config; },
  get reason() { return d.reason; },
  get busy() { return d.busy; },
  get vendorId() { return cfg().vendorId ?? 0; },
  get productId() { return cfg().productId ?? 0; },
  get product() { return cfg().product ?? d.product ?? ""; },
  get protocol() { return cfg().protocol ?? 0; },
  get layers() { return cfg().layers ?? 0; },
  get keymap() { return cfg().keymap ?? []; }, // [layer][row][col]
  get encoders() { return cfg().encoders ?? []; }, // [layer][encoder] = [ccw, cw]
  get values() { return cfg().values ?? {}; }, // "ch:id" -> number[]
  /** parsed VIA JSON (frontend copy), or null if the app has no definition for this keyboard */
  get definition() { return d.config ? definitionFor(cfg().vendorId, cfg().productId) : null; },
};

/** Send a command to the keyboard driver (`type` matches Rust `KbdCmd`, camelCase). */
export const sendKbd = (cmd) => send("via", cmd);

// ---- browser demo (?demo): the example macropad in keyboards/example-macropad.json ----
const base = [
  [0x5f, 0x60, 0x61, 0xa8], // 7 8 9 Mute (knob press)
  [0x5c, 0x5d, 0x5e, 0x56], // 4 5 6 -
  [0x59, 0x5a, 0x5b, 0x58], // 1 2 3 Enter
];
const fn = base.map((r) => r.map(() => 1)); // transparent

export const demo = {
  state: {
    product: "Example Macropad",
    serial: "",
    config: {
      vendorId: 0x0000, productId: 0x0000, product: "Example Macropad", protocol: 12, definition: "example-macropad", layers: 4,
      keymap: [base, fn, fn, fn].map((l) => l.map((r) => [...r])),
      encoders: Array.from({ length: 4 }, () => [[0xaa, 0xa9]]), // [ccw, cw]: turn left = Vol−
      values: { "3:1": [198], "3:2": [1], "3:3": [63], "3:4": [167, 52] },
    },
  },
  apply(d, c) {
    const k = d.config;
    if (c.type === "setKeycode") k.keymap[c.layer][c.row][c.col] = c.code;
    if (c.type === "setEncoder") k.encoders[c.layer][c.encoder][c.clockwise ? 1 : 0] = c.code;
    if (c.type === "setValue") k.values[`${c.channel}:${c.id}`] = c.data;
  },
};
