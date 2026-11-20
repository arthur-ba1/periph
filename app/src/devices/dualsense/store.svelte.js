// DualSense state: a view over the generic device registry.
// Rust side: crates/periph-app/src/drivers/dualsense.rs; contract in docs/dualsense-protocol.md.
import { deviceState, send as sendTo } from "$lib/device-state.svelte.js";

/** { connected, product, serial, reason, code, config, status, busy } fed by the "dualsense" driver.
 *  config: { emulation, forwardFeedback, hidePhysical }
 *  status: { virtualActive, batteryRaw, outputReports, hidHide: "missing" | "off" | "hiding" } */
export const ds = deviceState("dualsense");

/** Send a command. `type` matches the Rust `Cmd` enum (camelCase). */
export function send(type, value) {
  return sendTo("dualsense", value === undefined ? { type } : { type, value });
}

/** Input report byte 53: low nibble = level (0..10), high nibble = 0 discharging, 1 charging,
 *  2 full (same reading as Linux hid-playstation). */
export function battery(raw) {
  if (raw == null) return null;
  const level = raw & 0x0f, state = raw >> 4;
  if (state === 2) return { percent: 100, charging: false, full: true };
  return { percent: Math.min(level * 10 + 5, 100), charging: state === 1, full: false };
}

// ---- controller stage (controller-view.svelte) ----

/** Parts the stage can fly to (camera shots in controller-3d.svelte) and their config section. */
export const PARTS = {
  overview: {},
  lightbar: { section: "lightbar" },
  player: { section: "playerLeds" },
  mic: { section: "micLed" },
  triggers: {},
  l2: { section: "leftTrigger" },
  r2: { section: "rightTrigger" },
};

/** Shared stage state. focus: part key; flash: bumped on every change so the stage pulses the part that changed. */
export const ui = $state({ focus: "overview", flash: { part: null, n: 0 } });

/** Point the stage at a part. */
export function focusPart(part) {
  if (!PARTS[part]) return;
  ui.focus = part;
}

/** Pulse a part on the stage (called after a setting changes). */
export function flashPart(part) {
  ui.flash = { part, n: ui.flash.n + 1 };
}

/** Stage part for a config section ("lightbar" -> "lightbar", "leftTrigger" -> "l2"). */
export const partOf = (section) => Object.keys(PARTS).find((k) => PARTS[k].section === section) ?? "overview";

// ---- effects (Rust: drivers/dualsense/effects.rs, encoders in dualsense-proto effects.rs) ----

/** Trigger modes in menu order, with their default parameters. */
export const TRIGGER_MODES = [
  { kind: "off" },
  { kind: "feedback", start: 1, strength: 8 },
  { kind: "weapon", start: 2, end: 6, strength: 8 },
  { kind: "vibration", start: 1, amplitude: 8, frequency: 10 },
  { kind: "slope", start: 1, end: 9, startStrength: 8, endStrength: 1 },
  { kind: "multiFeedback", strengths: [8, 8, 8, 0, 0, 0, 8, 8, 0, 0] },
  { kind: "multiVibration", amplitudes: [8, 8, 8, 8, 8, 0, 0, 0, 8, 8], frequency: 10 },
  { kind: "bow", start: 1, end: 8, strength: 5, snap: 8 },
  { kind: "galloping", start: 1, end: 9, firstFoot: 3, secondFoot: 6, frequency: 4 },
  { kind: "machine", start: 1, end: 9, amplitudeA: 2, amplitudeB: 7, frequency: 10, period: 5 },
  { kind: "gameCube" },
  { kind: "custom", mode: 0x21, forces: [255, 255, 255, 255, 255, 255, 255] },
];

/** Slider parameters per mode: [key, min, max]. Ranges match the Rust validation. */
export const TRIGGER_PARAMS = {
  feedback: [["start", 0, 9], ["strength", 1, 8]],
  weapon: [["start", 2, 7], ["end", 3, 8], ["strength", 1, 8]],
  vibration: [["start", 0, 9], ["amplitude", 1, 8], ["frequency", 1, 10]],
  slope: [["start", 0, 8], ["end", 1, 9], ["startStrength", 1, 8], ["endStrength", 1, 8]],
  multiVibration: [["frequency", 1, 10]],
  bow: [["start", 0, 7], ["end", 1, 8], ["strength", 1, 8], ["snap", 1, 8]],
  galloping: [["start", 0, 8], ["end", 1, 9], ["firstFoot", 0, 6], ["secondFoot", 1, 7], ["frequency", 1, 10]],
  machine: [["start", 0, 8], ["end", 1, 9], ["amplitudeA", 0, 7], ["amplitudeB", 0, 7], ["frequency", 1, 10], ["period", 0, 10]],
};

/** Raw mode bytes for the custom effect. */
export const CUSTOM_MODES = [0x01, 0x05, 0x21, 0x22, 0x25];

/** Sets `key` and keeps the ordering rules the controller format needs (start < end, ...). */
export function withParam(effect, key, value) {
  const e = { ...effect, [key]: value };
  const pairs = [["start", "end"], ["firstFoot", "secondFoot"]];
  for (const [lo, hi] of pairs) {
    if (!(lo in e) || !(hi in e) || e[lo] < e[hi]) continue;
    const max = TRIGGER_PARAMS[e.kind].find(([k]) => k === hi)[2];
    if (key === lo) e[hi] = Math.min(e[lo] + 1, max);
    else e[lo] = Math.max(e[hi] - 1, 0);
  }
  return e;
}

/** Approximate strength per zone (0 = none, 1–8) for the preview bar; null when not zone-based. */
export function zonesOf(e) {
  if (!e) return null;
  const z = Array(10).fill(0);
  const span = (a, b, v) => { for (let i = a; i <= b; i++) z[i] = v; };
  switch (e.kind) {
    case "off": return z;
    case "feedback": span(e.start, 9, e.strength); return z;
    case "vibration": span(e.start, 9, e.amplitude); return z;
    case "weapon": span(e.start, e.end, e.strength); return z;
    case "bow": span(e.start, e.end, e.strength); return z;
    case "galloping": span(e.start, e.end, 4); return z;
    case "machine": span(e.start, e.end, Math.max(e.amplitudeA, e.amplitudeB) + 1); return z;
    case "slope":
      for (let i = e.start; i < 10; i++) z[i] = i <= e.end ? Math.round(e.startStrength + ((e.endStrength - e.startStrength) * (i - e.start)) / (e.end - e.start)) : e.endStrength;
      return z;
    case "multiFeedback": return [...e.strengths];
    case "multiVibration": return [...e.amplitudes];
    default: return null;
  }
}

/** Player LED patterns (bit 0 = leftmost LED), as the PlayStation shows player numbers. */
export const PLAYER_PATTERNS = [0x04, 0x0a, 0x15, 0x1b, 0x1f];

/** Lightbar colour presets. */
export const COLORS = [[255, 0, 0], [255, 120, 0], [255, 220, 0], [0, 255, 60], [0, 200, 255], [0, 80, 255], [140, 0, 255], [255, 0, 160], [255, 255, 255]];

export const toHex = (c) => "#" + c.map((v) => v.toString(16).padStart(2, "0")).join("");
export const fromHex = (h) => [1, 3, 5].map((i) => Number.parseInt(h.slice(i, i + 2), 16));

// ---- browser demo (?demo) ----
export const demo = {
  state: {
    product: "DualSense Wireless Controller",
    serial: "",
    config: {
      emulation: false, forwardFeedback: true, hidePhysical: true,
      lightbar: { mode: "breathing", color: [0, 80, 255], brightness: 100, speed: 5, override: false },
      playerLeds: { enabled: true, mask: 0x04, brightness: 0, override: false },
      micLed: { enabled: false, on: false, override: false },
      leftTrigger: { effect: null, override: false },
      rightTrigger: { effect: { kind: "weapon", start: 2, end: 6, strength: 8 }, override: false },
    },
    status: { virtualActive: false, batteryRaw: 0x07, outputReports: 0, hidHide: "off" },
  },
  apply(d, { type, value }) {
    const c = d.config, s = d.status;
    if (type === "emulation") {
      c.emulation = s.virtualActive = value;
      s.hidHide = value && c.hidePhysical ? "hiding" : "off";
      if (value) s.outputReports += 4;
    }
    if (type === "forwardFeedback") c.forwardFeedback = value;
    if (type === "hidePhysical") {
      c.hidePhysical = value;
      s.hidHide = value && c.emulation ? "hiding" : "off";
    }
    if (["lightbar", "playerLeds", "micLed", "leftTrigger", "rightTrigger"].includes(type)) c[type] = value;
  },
};
