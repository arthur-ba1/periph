// Centres in Periph model space. Several controls have separate caps and inserts.
const FACE = [
  ["triangle", 0.1218, -0.0828],
  ["circle", 0.1506, -0.054],
  ["cross", 0.1218, -0.0252],
  ["square", 0.0932, -0.054],
];
const DPAD = [
  ["up", -0.1213, -0.0736],
  ["right", -0.1026, -0.0535],
  ["down", -0.1226, -0.0346],
  ["left", -0.1415, -0.0548],
];

/** Locate separate pieces in the existing DualSense GLB. */
export function controlForPiece(material, { n, min, max }) {
  const x = (min[0] + max[0]) / 2, z = (min[2] + max[2]) / 2;
  const w = max[0] - min[0];
  const left = x < 0;
  if (n < 50 || material === "1002") return null;
  if (Math.abs(Math.abs(x) - 0.0622) < 0.004 && Math.abs(z) < 0.004 && min[1] > 0.05 && w < 0.06) {
    return left ? "l3" : "r3";
  }
  if (Math.abs(x) < 0.005 && w > 0.14 && w < 0.17 && z < -0.07 && min[1] > 0.03) return "touchpad";
  if (material !== "1001") return null;
  if (Math.abs(x) < 0.015 && w < 0.03 && min[1] > 0.044 && min[2] > -0.016 && max[2] < 0.008) return "ps";
  if (Math.abs(x) < 0.004 && w < 0.025 && z > 0.016 && z < 0.024 && min[1] > 0.035) return "mute";
  if (Math.abs(Math.abs(x) - 0.0914) < 0.004 && Math.abs(z + 0.0966) < 0.004 && w < 0.016 && min[1] > 0.05) {
    return left ? "share" : "options";
  }
  if (Math.abs(Math.abs(x) - 0.121) < 0.012 && min[2] < -0.11 && w < 0.07) {
    if (max[1] < 0.01 && min[1] < -0.02) return left ? "l2" : "r2";
    if (max[1] > 0.025 && max[1] < 0.05 && min[1] > -0.01) return left ? "l1" : "r1";
  }
  if (min[1] < 0.047 || w < 0.02 || w > 0.033) return null;
  return (left ? DPAD : FACE).find(([, cx, cz]) => Math.abs(x - cx) < 0.004 && Math.abs(z - cz) < 0.004)?.[0] ?? null;
}

const DIRECTIONS = [
  ["up"], ["up", "right"], ["right"], ["right", "down"],
  ["down"], ["down", "left"], ["left"], ["left", "up"],
];

export function controlPressed(buttons, part) {
  return buttons?.[part] === true || !!DIRECTIONS[buttons?.dpad]?.includes(part);
}

export const controlAxis = (value = 128) => (value - 128) / (value < 128 ? 128 : 127);

export function controlTrigger(input, part) {
  const index = part === "l2" ? 0 : 1;
  return Math.max((input?.triggers?.[index] ?? 0) / 255, input?.buttons?.[part] ? 0.12 : 0);
}

const STANDARD_BUTTONS = ["cross", "circle", "square", "triangle", "l1", "r1", "l2", "r2", "share", "options", "l3", "r3"];

/** In the browser demo, use the standard gamepad mapping when the browser exposes one. */
export function browserControls() {
  const pads = navigator.getGamepads?.();
  const available = Array.from(pads ?? []).filter((p) => p?.connected && p.mapping === "standard");
  const isDualSense = (p) => /dualsense|054c.*0ce6/i.test(p.id);
  const pad = available.find(isDualSense) ?? available[0];
  if (!pad) return null;
  const pressed = (index) => !!pad.buttons[index]?.pressed;
  const x = Number(pressed(15)) - Number(pressed(14));
  const y = Number(pressed(13)) - Number(pressed(12));
  const dpad = x || y ? Math.round(Math.atan2(x, -y) / (Math.PI / 4) + 8) % 8 : 8;
  const buttons = { dpad, ps: pressed(16) };
  if (isDualSense(pad)) buttons.touchpad = pressed(17);
  STANDARD_BUTTONS.forEach((key, i) => (buttons[key] = pressed(i)));
  const sticks = pad.axes.slice(0, 4).map((v) => Math.abs(v) <= 0.04 ? 128 : Math.round((v + 1) * 127.5));
  const triggers = [6, 7].map((i) => Math.round((pad.buttons[i]?.value ?? 0) * 255));
  return { buttons, sticks, triggers };
}
