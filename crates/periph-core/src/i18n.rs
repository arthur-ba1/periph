//! Translations for backend text (tray menu, errors shown in the UI).
//!
//! The catalogs are the same JSON files the frontend uses (`app/src/locales/<lang>.json` and
//! `app/src/devices/<id>/locales/<lang>.json`, the latter namespaced as `<id>.<key>`); the app
//! embeds them and calls [`load`] at startup. The UI tells the backend the chosen language with
//! [`set_locale`]. Lookup falls back to English, then to the key itself.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

struct State {
    locale: String,
    /// lang -> flattened key -> text
    catalogs: HashMap<String, HashMap<String, String>>,
}

fn state() -> &'static RwLock<State> {
    static S: OnceLock<RwLock<State>> = OnceLock::new();
    S.get_or_init(|| RwLock::new(State { locale: "en".into(), catalogs: HashMap::new() }))
}

fn flatten(prefix: &str, v: &Value, out: &mut HashMap<String, String>) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                flatten(&key, x, out);
            }
        }
        Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        _ => {}
    }
}

/// Add a JSON catalog for `lang`. `namespace` is "" for app-wide files or the device id.
pub fn load(lang: &str, namespace: &str, json: &str) -> Result<(), serde_json::Error> {
    let v: Value = serde_json::from_str(json)?;
    let mut s = state().write().unwrap();
    flatten(namespace, &v, s.catalogs.entry(lang.to_string()).or_default());
    Ok(())
}

/// Languages that have a catalog, e.g. ["en", "pt-BR"].
pub fn languages() -> Vec<String> {
    let mut v: Vec<String> = state().read().unwrap().catalogs.keys().cloned().collect();
    v.sort();
    v
}

pub fn set_locale(lang: &str) {
    state().write().unwrap().locale = lang.to_string();
}

pub fn locale() -> String {
    state().read().unwrap().locale.clone()
}

/// Translate `key` in the current language.
pub fn t(key: &str) -> String {
    let s = state().read().unwrap();
    for lang in [s.locale.as_str(), "en"] {
        if let Some(text) = s.catalogs.get(lang).and_then(|c| c.get(key)) {
            return text.clone();
        }
    }
    key.to_string()
}

/// Translate and fill `{name}` placeholders.
pub fn tf(key: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut text = t(key);
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), &value.to_string());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_falls_back_and_interpolates() {
        load("en", "", r#"{ "tray": { "open": "Open", "dpi": "DPI {n}" } }"#).unwrap();
        load("pt-BR", "", r#"{ "tray": { "open": "Abrir" } }"#).unwrap();
        load("en", "dev", r#"{ "x": "Device X" }"#).unwrap();
        set_locale("pt-BR");
        assert_eq!(t("tray.open"), "Abrir");
        assert_eq!(tf("tray.dpi", &[("n", &800)]), "DPI 800"); // English fallback
        assert_eq!(t("dev.x"), "Device X");
        assert_eq!(t("missing.key"), "missing.key");
        set_locale("en");
    }
}
