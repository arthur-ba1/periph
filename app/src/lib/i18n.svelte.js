// Translations. Catalogs are JSON files, one per language:
//   src/locales/<lang>.json                 app-wide keys        -> t("app.refresh")
//   src/devices/<id>/locales/<lang>.json    device keys          -> t("<id>.dpi.title")
// The Rust backend embeds the same files (tray menu, error messages), so one file per language
// translates everything. Each app-wide file names its language in "_name". See docs/translating.md.
import { invoke } from "@tauri-apps/api/core";

const app = import.meta.glob("../locales/*.json", { eager: true, import: "default" });
const dev = import.meta.glob("../devices/*/locales/*.json", { eager: true, import: "default" });

const flatten = (obj, prefix, out) => {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) flatten(v, key, out);
    else out[key] = v;
  }
  return out;
};

const catalogs = {}; // lang -> key -> text
const lang = (path) => path.split("/").pop().replace(/\.json$/, "");
for (const [path, json] of Object.entries(app)) flatten(json, "", (catalogs[lang(path)] ??= {}));
for (const [path, json] of Object.entries(dev)) {
  const id = path.split("/").at(-3);
  if (!id.startsWith("_")) flatten(json, id, (catalogs[lang(path)] ??= {}));
}

/** Languages with an app-wide catalog: [{ code: "pt-BR", name: "Português (Brasil)" }, ...] */
export const LANGUAGES = Object.keys(app)
  .map((p) => ({ code: lang(p), name: catalogs[lang(p)]._name ?? lang(p) }))
  .sort((a, b) => a.name.localeCompare(b.name));

const FALLBACK = "en";

/** Best supported language for a BCP 47 tag ("pt-PT" -> "pt-BR", "de" -> "en"). */
export function match(tag) {
  if (!tag) return FALLBACK;
  const codes = LANGUAGES.map((l) => l.code);
  const exact = codes.find((c) => c.toLowerCase() === tag.toLowerCase());
  if (exact) return exact;
  const base = tag.split("-")[0].toLowerCase();
  return codes.find((c) => c.split("-")[0].toLowerCase() === base) ?? FALLBACK;
}

const stored = globalThis.localStorage?.getItem("periph.locale");
export const i18n = $state({ locale: match(stored ?? globalThis.navigator?.language) });

/**
 * Translate `key`, filling `{name}` placeholders from `params`. Falls back to English, then to
 * the key itself, so a missing translation is visible but never breaks the UI. Reading
 * `i18n.locale` here makes every caller re-render when the language changes.
 */
export function t(key, params) {
  const text = catalogs[i18n.locale]?.[key] ?? catalogs[FALLBACK]?.[key] ?? key;
  if (!params || typeof text !== "string") return text;
  return text.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m));
}

/** Number formatting in the UI language (thousands separators etc.). */
export const fmt = (n, opts) => new Intl.NumberFormat(i18n.locale, opts).format(n);

export function setLocale(code) {
  i18n.locale = match(code);
  localStorage.setItem("periph.locale", i18n.locale);
}

// Keep <html lang> and the backend (tray, error messages) in sync.
$effect.root(() => {
  $effect(() => {
    document.documentElement.lang = i18n.locale;
    if (globalThis.window?.__TAURI_INTERNALS__) invoke("set_locale", { locale: i18n.locale }).catch(() => {});
  });
});

/** Every key of every catalog, for the completeness test. */
export const _catalogs = catalogs;
