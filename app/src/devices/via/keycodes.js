// QMK keycodes (VIA protocol 12 / QMK 0.19+ numbering): names and picker categories.

const B = {};
const add = (code, short, long = short) => (B[code] = { short, long });

add(0x00, "", "Nenhuma");
add(0x01, "▽", "Transparente (usa a camada de baixo)");
"ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").forEach((c, i) => add(0x04 + i, c));
"1234567890".split("").forEach((c, i) => add(0x1e + i, c));
[
  [0x28, "Enter"], [0x29, "Esc"], [0x2a, "⌫", "Backspace"], [0x2b, "Tab"], [0x2c, "Espaço"],
  [0x2d, "-"], [0x2e, "="], [0x2f, "["], [0x30, "]"], [0x31, "\\"], [0x32, "#", "Non-US #"], [0x33, ";"],
  [0x34, "'"], [0x35, "`"], [0x36, ","], [0x37, "."], [0x38, "/"], [0x39, "Caps", "Caps Lock"],
  [0x46, "PrtSc", "Print Screen"], [0x47, "ScrLk", "Scroll Lock"], [0x48, "Pause"], [0x49, "Ins", "Insert"],
  [0x4a, "Home"], [0x4b, "PgUp", "Page Up"], [0x4c, "Del", "Delete"], [0x4d, "End"], [0x4e, "PgDn", "Page Down"],
  [0x4f, "→", "Seta direita"], [0x50, "←", "Seta esquerda"], [0x51, "↓", "Seta baixo"], [0x52, "↑", "Seta cima"],
  [0x53, "NumLk", "Num Lock"], [0x54, "N/", "Numpad /"], [0x55, "N*", "Numpad *"], [0x56, "N-", "Numpad -"],
  [0x57, "N+", "Numpad +"], [0x58, "NEnt", "Numpad Enter"], [0x63, "N.", "Numpad ."], [0x64, "\\|", "Non-US \\"],
  [0x65, "Menu", "Menu de contexto"],
].forEach(([c, s, l]) => add(c, s, l));
for (let i = 1; i <= 12; i++) add(0x39 + i, `F${i}`);
for (let i = 13; i <= 24; i++) add(0x68 + i - 13, `F${i}`);
for (let i = 1; i <= 9; i++) add(0x58 + i, `N${i}`, `Numpad ${i}`);
add(0x62, "N0", "Numpad 0");
[
  [0xa5, "Power", "Desligar"], [0xa6, "Sleep", "Suspender"], [0xa7, "Wake", "Acordar"],
  [0xa8, "Mute", "Mudo"], [0xa9, "Vol+", "Volume +"], [0xaa, "Vol−", "Volume −"],
  [0xab, "Next", "Próxima faixa"], [0xac, "Prev", "Faixa anterior"], [0xad, "Stop", "Parar"],
  [0xae, "Play", "Play/Pause"], [0xaf, "Select", "Selecionar mídia"], [0xb0, "Eject", "Ejetar"],
  [0xb1, "Mail", "E-mail"], [0xb2, "Calc", "Calculadora"], [0xb3, "PC", "Meu computador"],
  [0xb4, "Search", "Pesquisar"], [0xb5, "WWW", "Página inicial"], [0xb6, "Back", "Navegador: voltar"],
  [0xb7, "Fwd", "Navegador: avançar"], [0xb8, "WStop", "Navegador: parar"], [0xb9, "Reload", "Navegador: recarregar"],
  [0xba, "Fav", "Favoritos"], [0xbb, "FF", "Avançar mídia"], [0xbc, "Rew", "Retroceder mídia"],
  [0xbd, "Bri+", "Brilho da tela +"], [0xbe, "Bri−", "Brilho da tela −"], [0xbf, "Ctrl P", "Painel de controle"],
  [0xc0, "Assist", "Assistente"], [0xc1, "Mission", "Mission Control (Mac)"], [0xc2, "Launch", "Launchpad (Mac)"],
  [0xcd, "M↑", "Mouse: cima"], [0xce, "M↓", "Mouse: baixo"], [0xcf, "M←", "Mouse: esquerda"], [0xd0, "M→", "Mouse: direita"],
  [0xd1, "M1", "Mouse: botão 1"], [0xd2, "M2", "Mouse: botão 2"], [0xd3, "M3", "Mouse: botão 3"],
  [0xd4, "M4", "Mouse: botão 4"], [0xd5, "M5", "Mouse: botão 5"],
  [0xd9, "W↑", "Roda: cima"], [0xda, "W↓", "Roda: baixo"], [0xdb, "W←", "Roda: esquerda"], [0xdc, "W→", "Roda: direita"],
  [0xe0, "Ctrl", "Ctrl esquerdo"], [0xe1, "Shift", "Shift esquerdo"], [0xe2, "Alt", "Alt esquerdo"], [0xe3, "Win", "Win esquerdo"],
  [0xe4, "RCtrl", "Ctrl direito"], [0xe5, "RShift", "Shift direito"], [0xe6, "RAlt", "Alt direito"], [0xe7, "RWin", "Win direito"],
].forEach(([c, s, l]) => add(c, s, l));

const RGB = [
  [0x7820, "RGB", "Iluminação liga/desliga"], [0x7821, "Efeito+", "Próximo efeito"], [0x7822, "Efeito−", "Efeito anterior"],
  [0x7823, "Matiz+", "Matiz +"], [0x7824, "Matiz−", "Matiz −"], [0x7825, "Sat+", "Saturação +"], [0x7826, "Sat−", "Saturação −"],
  [0x7827, "Luz+", "Brilho do LED +"], [0x7828, "Luz−", "Brilho do LED −"], [0x7829, "Vel+", "Velocidade +"], [0x782a, "Vel−", "Velocidade −"],
];
RGB.forEach(([c, s, l]) => add(c, s, l));
add(0x700b, "WinLk", "Trava da tecla Win");

const MOD_NAMES = [[0x01, "Ctrl"], [0x02, "Shift"], [0x04, "Alt"], [0x08, "Win"]];

/** { short, long } for any 16-bit keycode. */
export function describe(code, def) {
  if (B[code]) return B[code];
  const n = code & 0xff;
  const base = B[n]?.short ?? `0x${n.toString(16)}`;
  if (code >= 0x0100 && code <= 0x1fff) {
    const m = code >> 8;
    const mods = MOD_NAMES.filter(([b]) => m & b).map(([, s]) => (m & 0x10 ? "R" : "") + s);
    return { short: `${mods.join("+")}+${base}`, long: `${mods.join(" + ")} + ${B[n]?.long ?? base}` };
  }
  if (code >= 0x4000 && code <= 0x4fff) {
    const l = (code >> 8) & 0x0f;
    return { short: `LT${l}`, long: `Segurar: camada ${l} · Tocar: ${B[n]?.long ?? base}` };
  }
  const layerOps = [
    [0x5200, "TO", "Ir para camada"], [0x5220, "MO", "Segurar: camada"], [0x5240, "DF", "Camada padrão"],
    [0x5260, "TG", "Alternar camada"], [0x5280, "OSL", "Camada por um toque"], [0x52c0, "TT", "Tocar/segurar camada"],
  ];
  for (const [start, s, l] of layerOps) {
    if (code >= start && code < start + 0x20) return { short: `${s}(${code - start})`, long: `${l} ${code - start}` };
  }
  if (code >= 0x7700 && code < 0x7720) return { short: `M${code - 0x7700}`, long: `Macro ${code - 0x7700}` };
  if (code >= 0x7e00 && code < 0x7e40) {
    const ck = def?.customKeycodes?.[code - 0x7e00];
    return { short: ck?.shortName ?? `KB${code - 0x7e00}`, long: ck?.title ?? `Função do teclado ${code - 0x7e00}` };
  }
  if (code >= 0x7e40 && code < 0x7e60) return { short: `FW${code - 0x7e40}`, long: `Função do firmware ${code - 0x7e40}` };
  if (code >= 0x7000 && code < 0x7100) return { short: `Mg${(code & 0xff).toString(16)}`, long: `Magic 0x${(code & 0xff).toString(16)}` };
  if (code >= 0x7c00 && code < 0x7c20) return { short: "Boot", long: "Função de sistema (bloqueada)" };
  return { short: `0x${code.toString(16).toUpperCase()}`, long: `Keycode 0x${code.toString(16).toUpperCase()}` };
}

const range = (a, b) => Array.from({ length: b - a + 1 }, (_, i) => a + i);

/** Picker categories. Dangerous codes (bootloader, EEPROM reset) are never offered. */
export function categories(layers) {
  const L = range(0, Math.max(0, layers - 1));
  return [
    { id: "basic", label: "Básico", groups: [
      { label: "Letras", codes: range(0x04, 0x1d) },
      { label: "Números e símbolos", codes: [...range(0x1e, 0x27), ...range(0x2d, 0x38), 0x64] },
      { label: "Controle", codes: [0x29, 0x2b, 0x39, 0x2c, 0x28, 0x2a, 0x49, 0x4c, 0x4a, 0x4d, 0x4b, 0x4e, 0x52, 0x50, 0x51, 0x4f, 0x46, 0x47, 0x48, 0x65] },
      { label: "Modificadores", codes: range(0xe0, 0xe7) },
      { label: "Funções", codes: [...range(0x3a, 0x45), ...range(0x68, 0x73)] },
      { label: "Numpad", codes: [0x53, 0x54, 0x55, 0x56, 0x57, 0x58, ...range(0x59, 0x63)] },
    ] },
    { id: "media", label: "Mídia", groups: [
      { label: "Áudio e mídia", codes: [0xa8, 0xa9, 0xaa, 0xae, 0xab, 0xac, 0xad, 0xbb, 0xbc] },
      { label: "Sistema", codes: [0xbd, 0xbe, 0xb2, 0xb3, 0xb1, 0xb4, 0xb5, 0xc1, 0xc2, 0xa6] },
      { label: "Mouse", codes: [0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xcd, 0xce, 0xcf, 0xd0, 0xd9, 0xda] },
    ] },
    { id: "layers", label: "Camadas", groups: [
      { label: "Segurar (MO)", codes: L.map((l) => 0x5220 + l) },
      { label: "Alternar (TG)", codes: L.map((l) => 0x5260 + l) },
      { label: "Ir para (TO)", codes: L.map((l) => 0x5200 + l) },
    ] },
    { id: "light", label: "Iluminação", groups: [{ label: "RGB", codes: RGB.map(([c]) => c) }] },
    { id: "special", label: "Especial", groups: [
      { label: "Teclado", codes: [0x00, 0x01, 0x700b] },
      { label: "Macros", codes: range(0x7700, 0x770f) },
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
