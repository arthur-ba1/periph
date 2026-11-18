// VT2 Max state: a view over the generic device registry plus mouse-only UI state.
import { deviceState, send as sendTo } from "$lib/device-state.svelte.js";

/** { connected, product, serial, reason, config, status, busy } fed by the "vt2" Rust driver. */
export const vt2 = deviceState("vt2");

/** Send a command to the mouse. `type` matches the Rust `Cmd` enum (camelCase). */
export function send(type, value) {
  return sendTo("vt2", value === undefined ? { type } : { type, value });
}

// Shared UI state: the hero mouse lives in Hero.svelte and docks into the Buttons page slot.
// `settled`: the dock animation finished, so the Buttons page draws the mouse inside its own
// slot (scrolls natively) and the floating layer hides until the next transition.
export const ui = $state({ slot: null, sel: 6, settled: false });

// UI preferences (local only)
export const prefs = $state({ color: +(globalThis.localStorage?.getItem("vt2.color") ?? 4) });
export function setColor(c) {
  prefs.color = c;
  localStorage.setItem("vt2.color", String(c));
}

export const POLLING = [125, 250, 500, 1000, 2000, 4000, 8000];

export const BUTTONS = [
  { id: "left", name: "Botão esquerdo" },
  { id: "middle", name: "Clique da roda" },
  { id: "right", name: "Botão direito" },
  { id: "dpiUp", name: "DPI +" },
  { id: "dpiDown", name: "DPI −" },
  { id: "forward", name: "Lateral frente" },
  { id: "back", name: "Lateral trás" },
  { id: "wheelUp", name: "Roda para cima", locked: true },
  { id: "wheelDown", name: "Roda para baixo", locked: true },
  { id: "diy", name: "Botão DIY", hint: "embaixo" },
];

// ---- browser demo (?demo) ----
export const demo = {
  state: {
    product: "Rapoo Gaming Device",
    serial: "DEMO-0001",
    config: {
      polling: 2000, dpi: [400, 800, 1200, 1600, 3200, 6400, 26000], stageCount: 7, active: 1,
      lodMm: 1.1, motionSync: true, linear: true, ripple: false, glass: false, angle: 0,
      debouncePress: 4, debounceRelease: 4,
      buttons: [{ Mouse: 1 }, { Mouse: 4 }, { Mouse: 2 }, "DpiUp", "DpiDown", { Mouse: 16 }, { Mouse: 8 }, { Wheel: 8 }, { Wheel: 8 }, "Disabled"],
    },
    status: { dpi_stage: 1, dpi_x: 800, dpi_y: 800, battery: 83, polling_hz: 2000, wired: false, charging: false },
  },
  apply(d, { type, value }) {
    const c = d.config, s = d.status;
    if (type === "polling") s.polling_hz = c.polling = value;
    if (type === "activeStage") { c.active = s.dpi_stage = value; s.dpi_x = s.dpi_y = c.dpi[value]; }
    if (type === "stageDpi") { c.dpi[value[0]] = value[1]; if (value[0] === c.active) s.dpi_x = s.dpi_y = value[1]; }
  },
};
