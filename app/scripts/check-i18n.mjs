// Translation check: every language has the same keys as English, and every literal key used
// in the code (t("...") / tf("...") in .svelte, .js and .rs files) exists in English.
// Run: npm --prefix app run check:i18n  (also runs in CI)
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";

const root = new URL("../..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const src = join(root, "app", "src");

const walk = (dir, out = []) => {
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "dist" || name === "target") continue;
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
};

const flatten = (obj, prefix = "", out = {}) => {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) flatten(v, key, out);
    else out[key] = v;
  }
  return out;
};

// lang -> key -> value, merging app-wide and device catalogs like the app does
const catalogs = {};
for (const file of walk(src).filter((f) => f.endsWith(".json") && f.split(sep).includes("locales"))) {
  const parts = relative(src, file).split(sep);
  const lang = parts.at(-1).replace(/\.json$/, "");
  const ns = parts[0] === "devices" ? parts[1] : "";
  if (ns.startsWith("_")) continue;
  flatten(JSON.parse(readFileSync(file, "utf8")), ns, (catalogs[lang] ??= {}));
}

const errors = [];
const en = catalogs.en ?? {};
for (const [lang, cat] of Object.entries(catalogs)) {
  if (lang === "en") continue;
  for (const k of Object.keys(en)) if (!(k in cat)) errors.push(`${lang}: missing "${k}"`);
  for (const k of Object.keys(cat)) if (!(k in en)) errors.push(`${lang}: "${k}" is not in en`);
  for (const [k, v] of Object.entries(en)) {
    if (Array.isArray(v) && Array.isArray(cat[k]) && v.length !== cat[k].length) errors.push(`${lang}: "${k}" has ${cat[k].length} items, en has ${v.length}`);
    if (typeof v === "string" && typeof cat[k] === "string") {
      const ph = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(",");
      if (ph(v) !== ph(cat[k])) errors.push(`${lang}: "${k}" placeholders {${ph(cat[k])}} differ from en {${ph(v)}}`);
    }
  }
}

const code = [...walk(src), ...walk(join(root, "crates"))].filter((f) => /\.(svelte|js|rs)$/.test(f) && !f.includes(`${sep}templates${sep}`) && !/i18n.(rs|svelte.js)$/.test(f));
for (const file of code) {
  const text = readFileSync(file, "utf8");
  for (const m of text.matchAll(/\bt[f]?\(\s*"([A-Za-z][\w.]*)"/g)) {
    if (!(m[1] in en)) errors.push(`${relative(root, file)}: key "${m[1]}" not in en`);
  }
}

if (errors.length) {
  console.error(errors.join("\n"));
  console.error(`\n${errors.length} translation problem(s)`);
  process.exit(1);
}
console.log(`i18n ok: ${Object.keys(catalogs).length} languages, ${Object.keys(en).length} keys`);
