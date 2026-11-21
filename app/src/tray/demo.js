// Fake flyout payloads for UI work in the browser: ?tray (quick settings of the mouse),
// ?tray=menu, ?tray=pad, ?tray=kbd, ?tray=none (no device connected).
const mouse = {
  id: "vt2",
  kind: "mouse",
  name: "Rapoo VT2 Max",
  connected: true,
  header: "VT2 Max · wireless · battery 72% · 1600 DPI",
  battery: { percent: 72, charging: false },
  items: [
    { type: "submenu", label: "DPI", items: [400, 800, 1600, 3200].map((dpi, i) => ({ type: "check", id: `stage:${i}`, label: String(dpi), checked: i === 2 })) },
    {
      type: "submenu",
      label: "Polling rate",
      items: ["125 Hz", "250 Hz", "500 Hz", "1 kHz", "2 kHz", "4 kHz", "8 kHz"].map((label, i) => ({ type: "check", id: `poll:${i}`, label, checked: i === 3 })),
    },
    { type: "check", id: "motion", label: "Motion Sync", checked: true },
    { type: "check", id: "glass", label: "Glass mode", checked: false },
    { type: "action", id: "refresh", label: "Re-read configuration" },
  ],
};
const COLORS = [[255, 0, 0], [255, 120, 0], [255, 220, 0], [0, 255, 60], [0, 200, 255], [0, 80, 255], [140, 0, 255], [255, 0, 160], [255, 255, 255]];
const pad = {
  id: "dualsense",
  kind: "controller",
  name: "DualSense Wireless Controller",
  connected: true,
  header: "DualSense · Charging 45%",
  battery: { percent: 45, charging: true },
  items: [
    { type: "check", id: "emulation", label: "Virtual controller", checked: true },
    { type: "check", id: "hide", label: "Hide the physical controller", checked: true },
    { type: "check", id: "haptics", label: "Vibration", checked: true },
    { type: "check", id: "lightbar", label: "Lightbar", checked: true },
    { type: "colors", label: "Color", options: COLORS.map((rgb, i) => ({ id: `color:${i}`, rgb, checked: i === 5 })) },
    { type: "submenu", label: "Effect", items: ["Static", "Breathing", "Blink", "Rainbow"].map((label, i) => ({ type: "check", id: `mode:${i}`, label, checked: i === 0 })) },
    { type: "submenu", label: "Brightness", items: [25, 50, 75, 100].map((b) => ({ type: "check", id: `bright:${b}`, label: `${b}%`, checked: b === 100 })) },
    { type: "submenu", label: "Player LEDs", items: ["1", "2", "3", "4", "None", "All"].flatMap((label, i) => [...(i === 4 ? [{ type: "separator" }] : []), { type: "check", id: `leds:${i}`, label, checked: i === 0 }]) },
  ],
};
const KBD_COLORS = [[255, 0, 0], [255, 128, 0], [255, 230, 0], [0, 255, 60], [0, 200, 255], [40, 60, 255], [170, 0, 255], [255, 255, 255]];
const keyboard = {
  id: "via",
  kind: "keyboard",
  name: "AKKO 5075",
  connected: true,
  header: "AKKO 5075",
  battery: null,
  items: [
    {
      type: "submenu",
      label: "Effect",
      items: ["Off", "Solid color", "Breathing", "Color cycle", "Cycle left → right", "Cycle up → down", "Rainbow chevron", "Pinwheel", "Spiral", "Raindrops", "Typing heatmap", "Reactive"].map((label, i) => ({ type: "check", id: `effect:${i}`, label, checked: i === 2 })),
    },
    { type: "colors", label: "Color", options: KBD_COLORS.map((rgb, i) => ({ id: `color:${i}`, rgb, checked: i === 5 })) },
    { type: "slider", id: "speed", label: "Effect speed", value: 25, min: 0, max: 100, step: 5, unit: "%" },
    { type: "slider", id: "bright", label: "Brightness", value: 80, min: 0, max: 100, step: 5, unit: "%" },
  ],
};

export function demoPanel(search) {
  const mode = new URLSearchParams(search).get("tray");
  const devices = mode === "none" ? [mouse, pad, keyboard].map((d) => ({ ...d, connected: false, battery: null })) : [mouse, pad, keyboard];
  return structuredClone({
    seq: 1,
    mode: mode === "menu" ? "menu" : "quick",
    device: mode === "none" ? null : mode === "pad" ? "dualsense" : mode === "kbd" ? "via" : "vt2",
    devices,
    autostart: true,
    locale: null,
  });
}
