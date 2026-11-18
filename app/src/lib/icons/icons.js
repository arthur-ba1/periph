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
  logo: `<rect x="2.75" y="9.25" width="9.25" height="10" rx="2.25"/><path d="M5.6 12h3.5"/><path d="M17.6 4.75c-1.9 0-3.05 1.3-3.05 3.3v7.4c0 2 1.25 3.3 3.05 3.3s3.05-1.3 3.05-3.3v-7.4c0-2-1.15-3.3-3.05-3.3Z"/><path d="M17.6 5v3"/>`,
  mouse: `<path d="${SHELL}"/><path d="M12 6.75v2.5"/>`,

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
};
