// Minimal KLE (keyboard-layout-editor) parser for VIA layouts.
// Each key label's first line is "row,col"; line 10 may carry "e<N>" for encoders.
// Supports x/y offsets, w/h sizes and c (cap color); rotation is ignored.

export function parseLayout(rows) {
  const keys = [];
  let y = 0;
  let color = "#cccccc";
  for (const row of rows) {
    let x = 0;
    let w = 1, h = 1;
    for (const item of row) {
      if (typeof item === "object") {
        if (item.x) x += item.x;
        if (item.y) y += item.y;
        if (item.w) w = item.w;
        if (item.h) h = item.h;
        if (item.c) color = item.c;
        continue;
      }
      const lines = String(item).split("\n");
      const [r, c] = lines[0].split(",").map(Number);
      const enc = lines[9]?.match(/^e(\d+)$/);
      keys.push({ x, y, w, h, row: r, col: c, color, encoder: enc ? Number(enc[1]) : null });
      x += w;
      w = 1;
      h = 1;
    }
    y += 1;
  }
  const width = Math.max(...keys.map((k) => k.x + k.w));
  const height = Math.max(...keys.map((k) => k.y + k.h));
  return { list: keys, width, height };
}
