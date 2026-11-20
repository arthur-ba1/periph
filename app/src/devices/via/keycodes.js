// QMK keycodes (VIA protocol 12 / QMK 0.19+ numbering): names and picker categories.
// Names that depend on the language are translation keys (prefixed with "@", resolved in
// describe()); universal ones ("F5", "Page Up", "Numpad 7") are plain text.
import { t } from "$lib/i18n.svelte.js";

const B = {};
const add = (code, short, long = short) => (B[code] = { short, long });
const tr = (s) => (typeof s === "string" && s.startsWith("@") ? t(`via.keycodes.${s.slice(1)}`) : s);

add(0x00, "", "@none");
add(0x01, "▽", "@transparent");
"ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").forEach((c, i) => add(0x04 + i, c));
"1234567890".split("").forEach((c, i) => add(0x1e + i, c));
[
  [0x28, "Enter"], [0x29, "Esc"], [0x2a, "⌫", "Backspace"], [0x2b, "Tab"], [0x2c, "@spaceShort", "@space"],
  [0x2d, "-"], [0x2e, "="], [0x2f, "["], [0x30, "]"], [0x31, "\\"], [0x32, "#", "Non-US #"], [0x33, ";"],
  [0x34, "'"], [0x35, "`"], [0x36, ","], [0x37, "."], [0x38, "/"], [0x39, "Caps", "Caps Lock"],
  [0x46, "PrtSc", "Print Screen"], [0x47, "ScrLk", "Scroll Lock"], [0x48, "Pause"], [0x49, "Ins", "Insert"],
  [0x4a, "Home"], [0x4b, "PgUp", "Page Up"], [0x4c, "Del", "Delete"], [0x4d, "End"], [0x4e, "PgDn", "Page Down"],
  [0x4f, "→", "@right"], [0x50, "←", "@left"], [0x51, "↓", "@down"], [0x52, "↑", "@up"],
  [0x53, "NumLk", "Num Lock"], [0x54, "N/", "Numpad /"], [0x55, "N*", "Numpad *"], [0x56, "N-", "Numpad -"],
  [0x57, "N+", "Numpad +"], [0x58, "NEnt", "Numpad Enter"], [0x63, "N.", "Numpad ."], [0x64, "\\|", "Non-US \\"],
  [0x65, "Menu", "@menu"],
].forEach(([c, s, l]) => add(c, s, l));
for (let i = 1; i <= 12; i++) add(0x39 + i, `F${i}`);
for (let i = 13; i <= 24; i++) add(0x68 + i - 13, `F${i}`);
for (let i = 1; i <= 9; i++) add(0x58 + i, `N${i}`, `Numpad ${i}`);
add(0x62, "N0", "Numpad 0");
[
  [0xa5, "Power", "@power"], [0xa6, "Sleep", "@sleep"], [0xa7, "Wake", "@wake"],
  [0xa8, "Mute", "@mute"], [0xa9, "Vol+", "Volume +"], [0xaa, "Vol−", "Volume −"],
  [0xab, "Next", "@next"], [0xac, "Prev", "@prev"], [0xad, "Stop", "@stop"],
  [0xae, "Play", "Play/Pause"], [0xaf, "Select", "@select"], [0xb0, "Eject", "@eject"],
  [0xb1, "Mail", "@mail"], [0xb2, "Calc", "@calc"], [0xb3, "PC", "@pc"],
  [0xb4, "Search", "@search"], [0xb5, "WWW", "@home"], [0xb6, "Back", "@webBack"],
  [0xb7, "Fwd", "@webFwd"], [0xb8, "WStop", "@webStop"], [0xb9, "Reload", "@webReload"],
  [0xba, "Fav", "@favorites"], [0xbb, "FF", "@ff"], [0xbc, "Rew", "@rew"],
  [0xbd, "Bri+", "@briUp"], [0xbe, "Bri−", "@briDown"], [0xbf, "Ctrl P", "@controlPanel"],
  [0xc0, "Assist", "@assistant"], [0xc1, "Mission", "Mission Control (Mac)"], [0xc2, "Launch", "Launchpad (Mac)"],
  [0xcd, "M↑", "@msUp"], [0xce, "M↓", "@msDown"], [0xcf, "M←", "@msLeft"], [0xd0, "M→", "@msRight"],
  [0xd9, "W↑", "@whUp"], [0xda, "W↓", "@whDown"], [0xdb, "W←", "@whLeft"], [0xdc, "W→", "@whRight"],
  [0xe0, "Ctrl", "@lctrl"], [0xe1, "Shift", "@lshift"], [0xe2, "Alt", "@lalt"], [0xe3, "Win", "@lwin"],
  [0xe4, "RCtrl", "@rctrl"], [0xe5, "RShift", "@rshift"], [0xe6, "RAlt", "@ralt"], [0xe7, "RWin", "@rwin"],
].forEach(([c, s, l]) => add(c, s, l));
for (let i = 1; i <= 5; i++) B[0xd0 + i] = { short: `M${i}`, long: () => t("via.keycodes.msBtn", { n: i }) };

const RGB = [
  [0x7820, "RGB", "@rgbToggle"], [0x7821, "@rgbNextShort", "@rgbNext"], [0x7822, "@rgbPrevShort", "@rgbPrev"],
  [0x7823, "@hueUpShort", "@hueUp"], [0x7824, "@hueDownShort", "@hueDown"], [0x7825, "@satUpShort", "@satUp"], [0x7826, "@satDownShort", "@satDown"],
  [0x7827, "@valUpShort", "@valUp"], [0x7828, "@valDownShort", "@valDown"], [0x7829, "@spdUpShort", "@spdUp"], [0x782a, "@spdDownShort", "@spdDown"],
];
RGB.forEach(([c, s, l]) => add(c, s, l));
add(0x700b, "WinLk", "@winLock");

const MOD_NAMES = [[0x01, "Ctrl"], [0x02, "Shift"], [0x04, "Alt"], [0x08, "Win"]];

const named = (code) => {
  const b = B[code];
  return b && { short: tr(b.short), long: typeof b.long === "function" ? b.long() : tr(b.long) };
};

/** { short, long } for any 16-bit keycode, in the UI language. */
export function describe(code, def) {
  const own = named(code);
  if (own) return own;
  const n = code & 0xff;
  const inner = named(n);
  const base = inner?.short ?? `0x${n.toString(16)}`;
  if (code >= 0x0100 && code <= 0x1fff) {
    const m = code >> 8;
    const mods = MOD_NAMES.filter(([b]) => m & b).map(([, s]) => (m & 0x10 ? "R" : "") + s);
    return { short: `${mods.join("+")}+${base}`, long: `${mods.join(" + ")} + ${inner?.long ?? base}` };
  }
  if (code >= 0x4000 && code <= 0x4fff) {
    const l = (code >> 8) & 0x0f;
    return { short: `LT${l}`, long: t("via.keycodes.layerTap", { layer: l, key: inner?.long ?? base }) };
  }
  const layerOps = [[0x5200, "TO", "to"], [0x5220, "MO", "mo"], [0x5240, "DF", "df"], [0x5260, "TG", "tg"], [0x5280, "OSL", "osl"], [0x52c0, "TT", "tt"]];
  for (const [start, s, key] of layerOps) {
    if (code >= start && code < start + 0x20) return { short: `${s}(${code - start})`, long: t(`via.keycodes.${key}`, { n: code - start }) };
  }
  if (code >= 0x7700 && code < 0x7720) return { short: `M${code - 0x7700}`, long: t("via.keycodes.macro", { n: code - 0x7700 }) };
  if (code >= 0x7e00 && code < 0x7e40) {
    const ck = def?.customKeycodes?.[code - 0x7e00];
    return { short: ck?.shortName ?? `KB${code - 0x7e00}`, long: ck?.title ?? t("via.keycodes.kbFunction", { n: code - 0x7e00 }) };
  }
  if (code >= 0x7e40 && code < 0x7e60) return { short: `FW${code - 0x7e40}`, long: t("via.keycodes.fwFunction", { n: code - 0x7e40 }) };
  if (code >= 0x7000 && code < 0x7100) return { short: `Mg${(code & 0xff).toString(16)}`, long: `Magic 0x${(code & 0xff).toString(16)}` };
  if (code >= 0x7c00 && code < 0x7c20) return { short: "Boot", long: t("via.keycodes.system") };
  return { short: `0x${code.toString(16).toUpperCase()}`, long: `Keycode 0x${code.toString(16).toUpperCase()}` };
}

const range = (a, b) => Array.from({ length: b - a + 1 }, (_, i) => a + i);
const c = (key) => t(`via.categories.${key}`);

/** Picker categories. Dangerous codes (bootloader, EEPROM reset) are never offered. */
export function categories(layers) {
  const L = range(0, Math.max(0, layers - 1));
  return [
    { id: "basic", label: c("basic"), groups: [
      { label: c("letters"), codes: range(0x04, 0x1d) },
      { label: c("numbers"), codes: [...range(0x1e, 0x27), ...range(0x2d, 0x38), 0x64] },
      { label: c("control"), codes: [0x29, 0x2b, 0x39, 0x2c, 0x28, 0x2a, 0x49, 0x4c, 0x4a, 0x4d, 0x4b, 0x4e, 0x52, 0x50, 0x51, 0x4f, 0x46, 0x47, 0x48, 0x65] },
      { label: c("modifiers"), codes: range(0xe0, 0xe7) },
      { label: c("functions"), codes: [...range(0x3a, 0x45), ...range(0x68, 0x73)] },
      { label: c("numpad"), codes: [0x53, 0x54, 0x55, 0x56, 0x57, 0x58, ...range(0x59, 0x63)] },
    ] },
    { id: "media", label: c("media"), groups: [
      { label: c("audio"), codes: [0xa8, 0xa9, 0xaa, 0xae, 0xab, 0xac, 0xad, 0xbb, 0xbc] },
      { label: c("system"), codes: [0xbd, 0xbe, 0xb2, 0xb3, 0xb1, 0xb4, 0xb5, 0xc1, 0xc2, 0xa6] },
      { label: c("mouse"), codes: [0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xcd, 0xce, 0xcf, 0xd0, 0xd9, 0xda] },
    ] },
    { id: "layers", label: c("layers"), groups: [
      { label: c("hold"), codes: L.map((l) => 0x5220 + l) },
      { label: c("toggle"), codes: L.map((l) => 0x5260 + l) },
      { label: c("goto"), codes: L.map((l) => 0x5200 + l) },
    ] },
    { id: "light", label: c("lighting"), groups: [{ label: "RGB", codes: RGB.map(([code]) => code) }] },
    { id: "special", label: c("special"), groups: [
      { label: c("keyboard"), codes: [0x00, 0x01, 0x700b] },
      { label: c("macros"), codes: range(0x7700, 0x770f) },
    ] },
  ];
}

// KeyboardEvent.code -> keycode (for "press a key to assign")
const CODE = {
  Escape: 0x29, Tab: 0x2b, CapsLock: 0x39, Space: 0x2c, Enter: 0x28, Backspace: 0x2a, Insert: 0x49, Delete: 0x4c,
  Home: 0x4a, End: 0x4d, PageUp: 0x4b, PageDown: 0x4e, ArrowUp: 0x52, ArrowLeft: 0x50, ArrowDown: 0x51, ArrowRight: 0x4f,
  Minus: 0x2d, Equal: 0x2e, BracketLeft: 0x2f, BracketRight: 0x30, Backslash: 0x31, Semicolon: 0x33, Quote: 0x34,
  Backquote: 0x35, Comma: 0x36, Period: 0x37, Slash: 0x38, IntlBackslash: 0x64, PrintScreen: 0x46, ScrollLock: 0x47,
  Pause: 0x48, ContextMenu: 0x65, NumLock: 0x53, NumpadDivide: 0x54, NumpadMultiply: 0x55, NumpadSubtract: 0x56,
  NumpadAdd: 0x57, NumpadEnter: 0x58, NumpadDecimal: 0x63, Numpad0: 0x62,
  ControlLeft: 0xe0, ShiftLeft: 0xe1, AltLeft: 0xe2, MetaLeft: 0xe3, ControlRight: 0xe4, ShiftRight: 0xe5, AltRight: 0xe6, MetaRight: 0xe7,
};
for (let i = 0; i < 26; i++) CODE[`Key${String.fromCharCode(65 + i)}`] = 0x04 + i;
for (let i = 1; i <= 9; i++) { CODE[`Digit${i}`] = 0x1d + i; CODE[`Numpad${i}`] = 0x58 + i; }
CODE.Digit0 = 0x27;
for (let i = 1; i <= 12; i++) CODE[`F${i}`] = 0x39 + i;
export const keycodeFromEvent = (e) => CODE[e.code];
