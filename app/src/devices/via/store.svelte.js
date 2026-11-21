// VIA keyboard state: a view over the generic device registry ("via" Rust driver).
import { deviceState, send } from "$lib/device-state.svelte.js";
import { DEFINITIONS, definitionFor } from "./definitions.js";

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

/** Keyboard finish the views draw (sidebar card right click). Names are i18n keys via.colors.<key>. */
export const COLORS = [
  { key: "white", hex: "#f2f2ee" },
  { key: "black", hex: "#1f1f22" },
];
/** Keycap style for the lighting preview: "pudding" (translucent sides glow) or "solid" (light only
 *  between the keys and through the legends). Names are i18n keys via.caps.<key>. */
export const CAPS = ["pudding", "solid"];
export const prefs = $state({
  color: globalThis.localStorage?.getItem("via.color") ?? "white",
  caps: globalThis.localStorage?.getItem("via.caps") ?? "pudding",
});
export function setColor(c) {
  prefs.color = c;
  localStorage.setItem("via.color", c);
}
export function setCaps(c) {
  prefs.caps = c;
  localStorage.setItem("via.caps", c);
}

/** Send a command to the keyboard driver (`type` matches Rust `KbdCmd`, camelCase). */
export const sendKbd = (cmd) => send("via", cmd);

// ---- browser demo (?demo): the example macropad in keyboards/example-macropad.json ----
// `?demo&kbd=<definition id>` simulates another bundled definition instead (blank keymap unless
// DEMO_KEYMAPS has one), to preview a layout without the keyboard plugged in.
const base = [
  [0x5f, 0x60, 0x61, 0xa8], // 7 8 9 Mute (knob press)
  [0x5c, 0x5d, 0x5e, 0x56], // 4 5 6 -
  [0x59, 0x5a, 0x5b, 0x58], // 1 2 3 Enter
];
const fn = base.map((r) => r.map(() => 1)); // transparent

const F = (n) => 0x39 + n, L = (c) => 0x04 + c.charCodeAt(0) - 65, N = (d) => (d === 0 ? 0x27 : 0x1d + d);
const DEMO_KEYMAPS = {
  // ANSI 75%; matrix positions as in the AKKO 5075 layout (col 15 = right column, 0,15 = knob)
  "akko-5075": {
    0: { 0: 0x29, ...Object.fromEntries([...Array(12)].map((_, i) => [i + 1, F(i + 1)])), 13: 0x4c, 15: 0xa8 },
    1: { 0: 0x35, ...Object.fromEntries([...Array(10)].map((_, i) => [i + 1, N((i + 1) % 10)])), 11: 0x2d, 12: 0x2e, 13: 0x2a, 15: 0x4a },
    2: { 0: 0x2b, ...Object.fromEntries([..."QWERTYUIOP"].map((c, i) => [i + 1, L(c)])), 11: 0x2f, 12: 0x30, 13: 0x31, 15: 0x4b },
    3: { 0: 0x39, ...Object.fromEntries([..."ASDFGHJKL"].map((c, i) => [i + 1, L(c)])), 10: 0x33, 11: 0x34, 13: 0x28, 15: 0x4e },
    4: { 0: 0xe1, ...Object.fromEntries([..."ZXCVBNM"].map((c, i) => [i + 1, L(c)])), 8: 0x36, 9: 0x37, 10: 0x38, 13: 0xe5, 14: 0x52, 15: 0x4d },
    5: { 0: 0xe0, 1: 0xe3, 2: 0xe2, 5: 0x2c, 9: 0xe6, 10: 0x5221, 11: 0xe4, 13: 0x50, 14: 0x51, 15: 0x4f },
  },
};

function demoState() {
  const id = typeof location !== "undefined" ? new URLSearchParams(location.search).get("kbd") : null;
  const def = id && DEFINITIONS.find((x) => x.id === id);
  if (!def) {
    return {
      product: "Example Macropad",
      serial: "",
      config: {
        vendorId: 0x0000, productId: 0x0000, product: "Example Macropad", protocol: 12, definition: "example-macropad", layers: 4,
        keymap: [base, fn, fn, fn].map((l) => l.map((r) => [...r])),
        encoders: Array.from({ length: 4 }, () => [[0xaa, 0xa9]]), // [ccw, cw]: turn left = Vol−
        values: { "3:1": [198], "3:2": [1], "3:3": [63], "3:4": [167, 52] },
      },
    };
  }
  const { rows, cols } = def.matrix;
  const map = DEMO_KEYMAPS[id] ?? {};
  const layer = (l) => [...Array(rows)].map((_, r) => [...Array(cols)].map((_, c) => (l === 0 ? map[r]?.[c] ?? 0 : 1)));
  const encoders = 1 + Math.max(-1, ...def.keys.list.map((k) => k.encoder ?? -1));
  return {
    product: def.displayName ?? def.name,
    serial: "",
    config: {
      vendorId: def.vid, productId: def.pid, product: def.displayName ?? def.name, protocol: 12, definition: id, layers: 4,
      keymap: [0, 1, 2, 3].map(layer),
      encoders: Array.from({ length: 4 }, () => Array.from({ length: encoders }, () => [0xaa, 0xa9])),
      values: { "3:1": [198], "3:2": [3], "3:3": [63], "3:4": [167, 52] },
    },
  };
}

export const demo = {
  state: demoState(),
  apply(d, c) {
    const k = d.config;
    if (c.type === "setKeycode") k.keymap[c.layer][c.row][c.col] = c.code;
    if (c.type === "setEncoder") k.encoders[c.layer][c.encoder][c.clockwise ? 1 : 0] = c.code;
    if (c.type === "setValue") k.values[`${c.channel}:${c.id}`] = c.data;
  },
};
