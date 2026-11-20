# Translating Periph

Periph ships in English and Brazilian Portuguese. All UI text, the tray menu and the
user-facing backend messages come from JSON catalogs, one file per language:

| file | keys | used by |
| --- | --- | --- |
| `app/src/locales/<lang>.json` | app-wide: `app.*`, `common.*`, `kinds.*`, `tray.*`, `errors.*` | UI and Rust |
| `app/src/devices/<id>/locales/<lang>.json` | one device, read as `<id>.<key>` | UI and Rust |

The Rust app embeds the same files at build time (`crates/periph-app/build.rs`), so one set of
files translates the window, the tray menu and the error messages.

## Add a language

1. Copy every `en.json` to `<lang>.json` next to it (`app/src/locales/` and each
   `app/src/devices/*/locales/`). Use a BCP 47 code: `es`, `fr`, `de`, `pt-PT`, `zh-CN`.
2. Set `"_name"` in `app/src/locales/<lang>.json` to the language's own name ("Español").
   The language picker lists it automatically.
3. Translate the values, never the keys. Keep `{placeholders}` exactly as they are; you may move
   them inside the sentence.
4. Run the checker, then look at the UI:
   ```
   npm --prefix app run check:i18n
   npm --prefix app run dev        # http://localhost:5173/?demo, pick the language in the title bar
   ```

The language is picked automatically from the system (`pt-PT` falls back to `pt-BR`, unknown
languages to English) and can be changed in the title bar. Missing keys fall back to English.

## Rules for code

- No user-visible literal strings in components or drivers. In Svelte/JS:
  `import { t } from "$lib/i18n.svelte.js"` and `t("vt2.dpi.title")`, `t("app.searchingFor", { name })`.
  In Rust: `periph_core::t("vt2.tray.refresh")`, `periph_core::tf("vt2.tray.stage", &[("n", &1)])`.
- Call `t()` where the text is rendered (templates, getters, `$derived`), not in module-level
  constants, so the text follows language changes. For static tables use getters:
  `get label() { return t(\`vt2.tabs.${key}\`); }`.
- Device modules put their text in their own `locales/` folder and prefix keys with their id.
- Every key must exist in every language; `check:i18n` (also run in CI) fails otherwise, and also
  checks that placeholders match and that every literal key used in `.svelte`, `.js` and `.rs`
  files exists.
- Low-level protocol crates (`vt2-proto`, `vt2-device`, `via`) keep technical error messages in
  English. When a state matters to the user (asleep, not found), the driver returns
  `periph_core::Absent::new("asleep", e)` and the module shows a translated explanation based on
  `dev.code`.
- Firmware-provided names (VIA effect names like `SOLID_COLOR`, menu labels like `Brightness`)
  are translated by lookup (`via.lighting.effects.*`, `via.lighting.labels.*`) with the original
  as fallback.
