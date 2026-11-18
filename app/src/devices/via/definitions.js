// VIA keyboard definitions bundled with the app (same files the Rust worker embeds).
import { parseLayout } from "./kle.js";

const files = import.meta.glob("./keyboards/*.json", { eager: true, import: "default" });

const hex = (s) => parseInt(String(s).replace(/^0x/i, ""), 16);

export const DEFINITIONS = Object.entries(files).map(([path, json]) => ({
  id: path.split("/").pop().replace(/\.json$/, ""),
  ...json,
  vid: hex(json.vendorId),
  pid: hex(json.productId),
  keys: parseLayout(json.layouts.keymap),
}));

export function definitionFor(vid, pid) {
  return DEFINITIONS.find((d) => d.vid === vid && d.pid === pid) ?? null;
}
