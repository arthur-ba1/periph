// KeyboardEvent.code -> HID usage id (keyboard page), plus labels.
const map = {
  Enter: 0x28, Escape: 0x29, Backspace: 0x2a, Tab: 0x2b, Space: 0x2c, Minus: 0x2d, Equal: 0x2e,
  BracketLeft: 0x2f, BracketRight: 0x30, Backslash: 0x31, Semicolon: 0x33, Quote: 0x34,
  Backquote: 0x35, Comma: 0x36, Period: 0x37, Slash: 0x38, CapsLock: 0x39,
  PrintScreen: 0x46, ScrollLock: 0x47, Pause: 0x48, Insert: 0x49, Home: 0x4a, PageUp: 0x4b,
  Delete: 0x4c, End: 0x4d, PageDown: 0x4e, ArrowRight: 0x4f, ArrowLeft: 0x50, ArrowDown: 0x51,
  ArrowUp: 0x52, NumLock: 0x53, NumpadDivide: 0x54, NumpadMultiply: 0x55, NumpadSubtract: 0x56,
  NumpadAdd: 0x57, NumpadEnter: 0x58, NumpadDecimal: 0x63, IntlBackslash: 0x64, ContextMenu: 0x65,
};
for (let i = 0; i < 26; i++) map["Key" + String.fromCharCode(65 + i)] = 0x04 + i;
for (let i = 1; i <= 9; i++) map["Digit" + i] = 0x1d + i;
map.Digit0 = 0x27;
for (let i = 1; i <= 9; i++) map["Numpad" + i] = 0x58 + i;
map.Numpad0 = 0x62;
for (let i = 1; i <= 12; i++) map["F" + i] = 0x39 + i;
for (let i = 13; i <= 24; i++) map["F" + i] = 0x68 + (i - 13);

const names = {};
for (const [code, usage] of Object.entries(map)) {
  names[usage] = code.replace(/^Key/, "").replace(/^Digit/, "").replace(/^Numpad/, "Num ").replace(/^Arrow/, "Seta ");
}
Object.assign(names, { 0x2c: "Espaço", 0x28: "Enter", 0x29: "Esc", 0x2a: "Backspace" });

export const codeToUsage = (code) => map[code];
export const usageName = (u) => names[u] ?? `0x${u.toString(16).toUpperCase().padStart(2, "0")}`;
export const F_EXTRA = Array.from({ length: 12 }, (_, i) => 0x68 + i); // F13..F24
