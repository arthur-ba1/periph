// Periph icons — original icon set for the app.
// Grid 24×24, 2px safe margin, stroke 1.75 (set by <Icon>), round caps/joins.
// Language: the mouse shell silhouette is the brand motif; filled shapes mark the
// "active" part (e.g. which mouse button). Values are inner SVG markup.

const SHELL = "M12 3.25c-3.6 0-5.75 2.45-5.75 6.15v5.2c0 3.75 2.45 6.15 5.75 6.15s5.75-2.4 5.75-6.15v-5.2c0-3.7-2.15-6.15-5.75-6.15Z";
const SPLIT = "M12 3.5v6.25";
const DECK = "M6.5 9.75h11";

export const ICONS = {
  // brand
  // keycap + mouse: the app configures both (same mark as the app icon, app-icon.svg)
  // app logo: a "P" whose bowl is a ring (same shape as crates/periph-app/app-icon.svg)
  logo: `<path d="M7.5 19.75V10"/><circle cx="12.5" cy="10" r="5"/>`,
  mouse: `<path d="${SHELL}"/><path d="M12 6.75v2.5"/>`,
  // adaptive trigger: the curved L2/R2 paddle with a resistance mark
  trigger: `<path d="M7.25 20.25V10.5c0-3.3 2.15-5.75 4.75-5.75s4.75 2.45 4.75 5.75v9.75"/><path d="M7.25 12.5h9.5"/><path d="M10 16.25h4"/>`,
  // controller: rounded body with two grips, d-pad cross and two face buttons
  gamepad: `<path d="M7.6 6.75h8.8c2.05 0 3.55 1.35 3.95 3.4l1 5.55c.4 2.15-1.95 3.55-3.45 2.05l-2.15-2.1h-7.5l-2.15 2.1c-1.5 1.5-3.85.1-3.45-2.05l1-5.55c.4-2.05 1.9-3.4 3.95-3.4Z"/><path d="M8 9.75v3.5M6.25 11.5h3.5"/><circle cx="15.4" cy="10.6" r=".95" fill="currentColor" stroke="none"/><circle cx="17.4" cy="12.6" r=".95" fill="currentColor" stroke="none"/>`,

  // controller parts (DualSense pages)
  // touchpad flanked by the two curved light strips
  lightbar: `<rect x="7.75" y="5.25" width="8.5" height="8" rx="1.75"/><path d="M4.75 5.5c-.45 3 0 5.75 1.6 8.5M19.25 5.5c.45 3 0 5.75-1.6 8.5"/><path d="M9.5 17.75h5"/>`,
  // row of five player LEDs in their slot
  playerLeds: `<rect x="2.75" y="8.75" width="18.5" height="6.5" rx="3.25"/><path d="M6.4 12h.01M9.2 12h.01M12 12h.01M14.8 12h.01M17.6 12h.01"/>`,
  mic: `<rect x="9.25" y="3.75" width="5.5" height="10" rx="2.75"/><path d="M6.25 11.25a5.75 5.75 0 0 0 11.5 0"/><path d="M12 17v3.25"/>`,
  // emulation: the controller with a dashed virtual twin behind it
  emulate: `<path d="M9.75 4.75h6.6c1.55 0 2.7 1 3 2.55l.55 3" stroke-dasharray="1.6 2.2"/><path d="M6.6 9.25h7.8c1.8 0 3.15 1.2 3.5 3l.85 4.85c.35 1.9-1.7 3.1-3.05 1.8l-1.9-1.85H7.2L5.3 18.9c-1.35 1.3-3.4.1-3.05-1.8l.85-4.85c.35-1.8 1.7-3 3.5-3Z"/><path d="M7 12v2.75M5.6 13.4h2.8"/><circle cx="13.75" cy="13.1" r=".9" fill="currentColor" stroke="none"/>`,

  // navigation
  dpi: `<path d="M4.75 16.75a7.75 7.75 0 1 1 14.5 0"/><path d="M12 14.25 15.25 9.5"/><circle cx="12" cy="14.25" r="1.35" fill="currentColor" stroke="none"/><path d="M7 11.1l.9.5M12 6.5v1M17 11.1l-.9.5"/>`,
  performance: `<path d="M4.75 19.25v-3.5"/><path d="M9.6 19.25v-6.5"/><path d="M14.4 19.25v-9.5"/><path d="M19.25 19.25V4.75"/>`,
  sensor: `<circle cx="12" cy="12" r="6.25"/><circle cx="12" cy="12" r="2.1" fill="currentColor" stroke="none"/><path d="M12 2.75v2.5M12 18.75v2.5M2.75 12h2.5M18.75 12h2.5"/>`,
  buttons: `<path d="${SHELL}"/><path d="${SPLIT}"/><path d="${DECK}"/><path d="M2.75 5.25l1.6 1M5.25 2.75l1 1.6"/>`,

  // mouse actions
  clickLeft: `<path d="${SHELL}"/><path d="M12 3.6c-3.15.1-5.2 2.3-5.4 5.6v.55H12Z" fill="currentColor" stroke="none"/><path d="${SPLIT}"/>`,
  clickRight: `<path d="${SHELL}"/><path d="M12 3.6c3.15.1 5.2 2.3 5.4 5.6v.55H12Z" fill="currentColor" stroke="none"/><path d="${SPLIT}"/>`,
  clickMiddle: `<path d="${SHELL}"/><rect x="10.75" y="5.75" width="2.5" height="4.5" rx="1.25" fill="currentColor" stroke="none"/>`,
  back: `<path d="M9.75 6.25 4.75 11.5l5 5.25"/><path d="M4.75 11.5h9.5a5 5 0 0 1 5 5v1.75"/>`,
  forward: `<path d="M14.25 6.25l5 5.25-5 5.25"/><path d="M19.25 11.5h-9.5a5 5 0 0 0-5 5v1.75"/>`,
  dpiUp: `<path d="M4.75 17.25a7.75 7.75 0 1 1 14.5 0"/><path d="M12 10.25v5.5M9.25 13h5.5"/>`,
  dpiDown: `<path d="M4.75 17.25a7.75 7.75 0 1 1 14.5 0"/><path d="M9.25 13h5.5"/>`,
  disabled: `<circle cx="12" cy="12" r="7.25"/><path d="M6.9 17.1 17.1 6.9"/>`,
  keyboard: `<rect x="2.75" y="6.25" width="18.5" height="11.5" rx="2.75"/><path d="M6.75 10h.01M10.25 10h.01M13.75 10h.01M17.25 10h.01M8.25 14h7.5"/>`,

  // system
  refresh: `<path d="M19.25 12a7.25 7.25 0 1 1-2.12-5.13"/><path d="M19.25 4.75v4.5h-4.5"/>`,
  minimize: `<path d="M6.75 12h10.5"/>`,
  close: `<path d="M7.25 7.25l9.5 9.5M16.75 7.25l-9.5 9.5"/>`,
  lock: `<rect x="5.25" y="10.75" width="13.5" height="9.5" rx="2.25"/><path d="M8.25 10.75V8.5a3.75 3.75 0 0 1 7.5 0v2.25"/><path d="M12 14.5v2"/>`,
  warning: `<path d="M10.3 4.6 3.4 17.1a1.95 1.95 0 0 0 1.7 2.9h13.8a1.95 1.95 0 0 0 1.7-2.9L13.7 4.6a1.95 1.95 0 0 0-3.4 0Z"/><path d="M12 9.75v4M12 16.9h.01"/>`,
  info: `<circle cx="12" cy="12" r="8.25"/><path d="M12 11v5M12 7.9h.01"/>`,
  light: `<circle cx="12" cy="12" r="3.5"/><path d="M12 3.75v1.5M12 18.75v1.5M3.75 12h1.5M18.75 12h1.5M6.2 6.2l1.05 1.05M16.75 16.75l1.05 1.05M6.2 17.8l1.05-1.05M16.75 7.25l1.05-1.05"/>`,
  palette: `<circle cx="12" cy="12" r="8.25"/><circle cx="12" cy="12" r="3" fill="currentColor" stroke="none"/>`,
  // language picker: globe with meridian and parallels
  language: `<circle cx="12" cy="12" r="8.25"/><path d="M3.75 12h16.5M12 3.75c2.2 2.3 3.3 5 3.3 8.25s-1.1 5.95-3.3 8.25M12 3.75c-2.2 2.3-3.3 5-3.3 8.25s1.1 5.95 3.3 8.25"/>`,
};
