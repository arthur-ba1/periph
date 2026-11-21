//! Loading and validating a package folder.

use crate::checksum;
use crate::spec::{Device, Field, Manifest, ReportKind, Setting, Transport};
use crate::template::{Template, Token};
use crate::value::{Resolved, Ty};
use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const FORMAT: u32 = 1;

#[derive(Debug)]
pub struct Package {
    pub dir: PathBuf,
    pub manifest: Manifest,
    pub device: Device,
    /// The files as written, for the UI (which renders the settings itself).
    pub manifest_json: Value,
    pub device_json: Value,
    /// lang -> catalog
    pub locales: BTreeMap<String, Value>,
    /// `transport.response.pendingCode`, kept for `Absent` (which needs a `'static` code).
    pub pending_code: Option<&'static str>,
}

fn read_json(p: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", p.display()))
}

const SETTING_KEYS: &[&str] = &[
    "id", "at", "type", "bit", "invert", "equals", "count", "values", "valuesFrom", "linear", "range", "unit", "write", "readOnly",
    "indexOf", "countFrom", "control", "from", "send", "default",
];
const STATUS_FIELD_KEYS: &[&str] = &["id", "at", "type", "bit", "invert", "equals", "count", "values", "valuesFrom", "linear", "range", "unit"];

/// Flatten a catalog into dotted keys.
pub fn flatten(prefix: &str, v: &Value, out: &mut BTreeMap<String, Value>) {
    match v {
        Value::Object(o) => o.iter().for_each(|(k, x)| flatten(&if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") }, x, out)),
        _ => {
            out.insert(prefix.to_string(), v.clone());
        }
    }
}

impl Package {
    /// Load a package folder. Structural errors fail here; see [`Package::problems`] for the rest.
    pub fn load(dir: &Path) -> Result<Package> {
        let manifest_json = read_json(&dir.join("manifest.json"))?;
        let device_json = read_json(&dir.join("device.json"))?;
        let manifest: Manifest = serde_json::from_value(manifest_json.clone()).context("manifest.json")?;
        let device: Device = serde_json::from_value(device_json.clone()).context("device.json")?;
        let mut locales = BTreeMap::new();
        if let Ok(rd) = std::fs::read_dir(dir.join("locales")) {
            for e in rd.filter_map(Result::ok) {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "json") {
                    let lang = p.file_stem().unwrap().to_string_lossy().to_string();
                    locales.insert(lang, read_json(&p)?);
                }
            }
        }
        let pending_code = match &device.transport {
            Transport::Registers(r) => r.response.pending_code.clone().map(|c| &*Box::leak(c.into_boxed_str())),
            Transport::Reports(_) => None,
        };
        let pkg = Package { dir: dir.to_path_buf(), manifest, device, manifest_json, device_json, locales, pending_code };
        let problems = pkg.problems();
        if !problems.is_empty() {
            bail!("{}:\n  {}", dir.display(), problems.join("\n  "));
        }
        Ok(pkg)
    }

    pub fn setting(&self, id: &str) -> Option<&Setting> {
        self.device.settings.iter().find(|s| s.id == id)
    }

    pub fn resolve<'a>(&'a self, f: &'a Field) -> Result<Resolved<'a>> {
        let ty = Ty::of(f, &self.device.codecs).map_err(|e| anyhow!(e))?;
        let values = match (&f.values, &f.values_from) {
            (Some(v), _) => Some(v.as_slice()),
            (None, Some(from)) => Some(
                self.setting(from).and_then(|s| s.field.values.as_deref()).ok_or_else(|| anyhow!("valuesFrom \"{from}\": no such setting with values"))?,
            ),
            (None, None) => None,
        };
        Ok(Resolved { field: f, ty, values, codecs: &self.device.codecs })
    }

    /// Everything wrong with the package, empty when it is fine.
    pub fn problems(&self) -> Vec<String> {
        let mut p = Vec::new();
        let m = &self.manifest;
        let d = &self.device;
        if m.format != FORMAT {
            p.push(format!("manifest: format {} is not supported (expected {FORMAT})", m.format));
        }
        if m.id.is_empty() || !m.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            p.push(format!("manifest: id \"{}\" must be lowercase letters, digits and '-'", m.id));
        }
        if self.dir.file_name().is_some_and(|n| n.to_string_lossy() != m.id) {
            p.push(format!("manifest: id \"{}\" must match the folder name", m.id));
        }
        if m.products.is_empty() {
            p.push("manifest: no products".into());
        }
        let iface = |name: &str, p: &mut Vec<String>, what: &str| {
            if !m.interfaces.contains_key(name) {
                p.push(format!("{what}: interface \"{name}\" is not in manifest.interfaces"));
            }
        };
        let ti = d.transport.interface();
        iface(ti, &mut p, "transport");
        if m.interfaces.get(ti).is_some_and(|i| i.optional) {
            p.push("transport: its interface can not be optional".into());
        }
        match &d.transport {
            Transport::Registers(r) => {
                for t in [&r.read, &r.write] {
                    if let Some(v) = t.vars().find(|v| !matches!(*v, "addr" | "len")) {
                        p.push(format!("transport: unknown variable {{{v}}} (use addr, len)"));
                    }
                }
                if !r.write.has_data() {
                    p.push("transport.write: needs {data}".into());
                }
                if r.max_chunk == 0 || r.report_length == 0 {
                    p.push("transport: maxChunk and reportLength must be > 0".into());
                }
                if r.response.ready.is_empty() || r.response.status + 1 >= r.report_length || r.response.data + 1 >= r.report_length {
                    p.push("transport.response: needs ready values and offsets inside the report".into());
                }
                if !d.reads.is_empty() {
                    p.push("reads: only for the reports transport".into());
                }
            }
            Transport::Reports(r) => {
                if r.report_length < 2 {
                    p.push("transport: reportLength must be at least 2".into());
                }
                if r.send == ReportKind::Input {
                    p.push("transport.send: output or feature".into());
                }
                if let Some(c) = &r.checksum {
                    if let Err(e) = checksum::validate(c, r.report_length) {
                        p.push(format!("transport.checksum: {e}"));
                    }
                }
                let fixed = |t: &Template, what: String, p: &mut Vec<String>| {
                    if t.has_data() || t.vars().next().is_some() {
                        p.push(format!("{what}: no variables allowed here"));
                    } else if t.fixed_len() > r.report_length {
                        p.push(format!("{what}: longer than reportLength"));
                    }
                };
                for (i, t) in r.after_write.iter().enumerate() {
                    fixed(t, format!("transport.afterWrite[{i}]"), &mut p);
                }
                let mut seen = BTreeSet::new();
                for rd in &d.reads {
                    if !seen.insert(rd.id.as_str()) {
                        p.push(format!("read {}: duplicate id", rd.id));
                    }
                    fixed(&rd.request, format!("read {}.request", rd.id), &mut p);
                    if rd.response.from == ReportKind::Output {
                        p.push(format!("read {}.response.from: feature or input", rd.id));
                    }
                }
            }
        }

        for (name, c) in &d.codecs {
            for v in &c.variants {
                if v.bytes.has_data() || v.bytes.vars().any(|x| x != "value") || v.bytes.fixed_len() != c.size {
                    p.push(format!("codec {name}.{}: bytes must be {} bytes with at most a {{value}}", v.kind, c.size));
                }
                let takes = v.bytes.vars().any(|x| x == "value");
                if takes && !v.read_only && v.values.is_none() && v.range.is_none() {
                    p.push(format!("codec {name}.{}: a writable {{value}} needs values or range", v.kind));
                }
            }
        }

        let mut ids = BTreeSet::new();
        for (i, s) in d.settings.iter().enumerate() {
            let w = format!("setting {}", s.id);
            if !ids.insert(s.id.as_str()) {
                p.push(format!("{w}: duplicate id"));
            }
            if let Some(o) = self.device_json["settings"][i].as_object() {
                for k in o.keys().filter(|k| !SETTING_KEYS.contains(&k.as_str())) {
                    p.push(format!("{w}: unknown key \"{k}\""));
                }
            }
            let r = match self.resolve(&s.field) {
                Ok(r) => r,
                Err(e) => {
                    p.push(format!("{w}: {e}"));
                    continue;
                }
            };
            let size = r.ty.size(&d.codecs);
            let writable = match &d.transport {
                Transport::Registers(_) => {
                    if s.at.is_empty() {
                        p.push(format!("{w}: no address"));
                    }
                    if s.from.is_some() || !s.send.is_empty() || s.default.is_some() {
                        p.push(format!("{w}: from, send and default are for the reports transport"));
                    }
                    !s.read_only
                }
                Transport::Reports(rp) => {
                    if s.write.is_some() {
                        p.push(format!("{w}: write blocks are for the registers transport (use send)"));
                    }
                    match &s.from {
                        Some(f) => match d.reads.iter().find(|r| &r.id == f) {
                            None => p.push(format!("{w}: from \"{f}\" is not a read")),
                            Some(_) if s.at.len() != 1 => p.push(format!("{w}: needs one offset in \"at\"")),
                            _ => {}
                        },
                        None if s.send.is_empty() => p.push(format!("{w}: needs from (to read it), send (to write it) or both")),
                        None => {}
                    }
                    for (k, t) in s.send.iter().enumerate() {
                        if let Some(v) = t.vars().find(|v| !matches!(*v, "value" | "index")) {
                            p.push(format!("{w}.send[{k}]: unknown variable {{{v}}} (use value, index)"));
                        }
                        if t.has_data() {
                            p.push(format!("{w}.send[{k}]: {{data}} is for the registers transport"));
                        }
                        if t.vars().any(|v| v == "index") && s.field.count.is_none() {
                            p.push(format!("{w}.send[{k}]: {{index}} needs an array (count)"));
                        }
                        if !t.vars().any(|v| v == "value") {
                            p.push(format!("{w}.send[{k}]: needs {{value}}"));
                        }
                        let len = t.0.iter().map(|tok| match tok {
                            Token::Var(n, _) if n == "value" => size,
                            Token::Var(_, wd) => wd.size(),
                            Token::Lit(_) => 1,
                            Token::Data => 0,
                        }).sum::<usize>();
                        if len > rp.report_length {
                            p.push(format!("{w}.send[{k}]: longer than reportLength"));
                        }
                    }
                    if r.ty.is_bool() && matches!(r.ty, Ty::Bit(_)) && s.from.is_none() && !s.send.is_empty() {
                        p.push(format!("{w}: a bit can only be written when it is also read (from)"));
                    }
                    !s.read_only && !s.send.is_empty()
                }
            };
            let numeric = !r.ty.is_bool() && !matches!(r.ty, Ty::Codec(_));
            if numeric && writable && r.values.is_none() && s.field.range.is_none() && s.index_of.is_none() {
                p.push(format!("{w}: a writable number needs values or range (or readOnly)"));
            }
            for (key, target) in [("indexOf", &s.index_of), ("countFrom", &s.count_from)] {
                if let Some(t) = target {
                    match self.setting(t) {
                        None => p.push(format!("{w}: {key} \"{t}\" is not a setting")),
                        Some(o) if key == "indexOf" && o.field.count.is_none() => p.push(format!("{w}: indexOf \"{t}\" is not an array")),
                        _ => {}
                    }
                }
            }
            if let Some(blocks) = &s.write {
                let size = r.ty.size(&d.codecs);
                for &a in &s.at {
                    if !blocks.iter().any(|b| a >= b.at && a + size as u32 <= b.at + b.len as u32) {
                        p.push(format!("{w}: address 0x{a:04X} is outside every write block"));
                    }
                }
            }
            if let Transport::Registers(_) = &d.transport {
                let total = size * s.field.count.unwrap_or(1);
                if !r.readable(&s.at, total, &r_spans(&d.transport)) {
                    p.push(format!("{w}: outside transport.readable"));
                }
            }
        }

        if let Some(st) = &d.status {
            iface(&st.interface, &mut p, "status");
            for (i, f) in st.fields.iter().enumerate() {
                if let Err(e) = self.resolve(&f.field) {
                    p.push(format!("status {}: {e}", f.id));
                }
                if let Some(o) = self.device_json["status"]["fields"][i].as_object() {
                    for k in o.keys().filter(|k| !STATUS_FIELD_KEYS.contains(&k.as_str())) {
                        p.push(format!("status {}: unknown key \"{k}\"", f.id));
                    }
                }
            }
        }
        let status_ids: BTreeSet<&str> = d.status.iter().flat_map(|s| s.fields.iter().map(|f| f.id.as_str())).collect();
        if let Some(tray) = &d.tray {
            if let Some(b) = &tray.battery {
                for f in std::iter::once(&b.level).chain(&b.charging) {
                    if !status_ids.contains(f.as_str()) {
                        p.push(format!("tray.battery: \"{f}\" is not a status field"));
                    }
                }
            }
            for it in &tray.items {
                match (&it.setting, it.refresh) {
                    (Some(s), false) if self.setting(s).is_none() => p.push(format!("tray: \"{s}\" is not a setting")),
                    (Some(_), false) | (None, true) => {}
                    _ => p.push("tray: an item is either a setting or refresh".into()),
                }
            }
        }
        for page in &d.pages {
            for s in &page.settings {
                if self.setting(s).is_none() {
                    p.push(format!("page {}: \"{s}\" is not a setting", page.id));
                }
            }
        }
        p.extend(self.locale_problems());
        p
    }

    /// Keys the UI and tray will look up, which must exist in English.
    fn required_keys(&self) -> Vec<String> {
        let d = &self.device;
        let mut k = Vec::new();
        for page in &d.pages {
            k.push(format!("pages.{}", page.id));
            for s in &page.settings {
                k.push(format!("settings.{s}.label"));
            }
        }
        for s in &d.settings {
            if let Some(Ty::Codec(c)) = Ty::of(&s.field, &d.codecs).ok() {
                for v in &d.codecs[&c].variants {
                    k.push(format!("codecs.{c}.{}", v.kind));
                }
            }
        }
        if let Some(tray) = &d.tray {
            k.extend(tray.header.clone());
            for it in &tray.items {
                match (&it.label, &it.setting) {
                    (Some(l), _) => k.push(l.clone()),
                    (None, Some(s)) => k.push(format!("settings.{s}.label")),
                    _ => {}
                }
            }
        }
        k.sort();
        k.dedup();
        k
    }

    fn locale_problems(&self) -> Vec<String> {
        let mut p = Vec::new();
        let flat = |v: &Value| {
            let mut out = BTreeMap::new();
            flatten("", v, &mut out);
            out
        };
        let Some(en) = self.locales.get("en").map(flat) else {
            return vec!["locales/en.json is required".into()];
        };
        for k in self.required_keys() {
            if !en.contains_key(&k) {
                p.push(format!("locales/en.json: missing \"{k}\""));
            }
        }
        for (lang, cat) in &self.locales {
            if lang == "en" {
                continue;
            }
            let cat = flat(cat);
            for k in en.keys().filter(|k| !cat.contains_key(*k)) {
                p.push(format!("locales/{lang}.json: missing \"{k}\""));
            }
            for k in cat.keys().filter(|k| !en.contains_key(*k)) {
                p.push(format!("locales/{lang}.json: \"{k}\" is not in en"));
            }
        }
        p
    }
}

fn r_spans(t: &Transport) -> Vec<(u32, u32)> {
    match t {
        Transport::Registers(r) => r.readable.iter().map(|s| (s.from, s.to)).collect(),
        Transport::Reports(_) => Vec::new(),
    }
}

impl Resolved<'_> {
    fn readable(&self, at: &[u32], len: usize, spans: &[(u32, u32)]) -> bool {
        spans.is_empty() || at.iter().all(|&a| spans.iter().any(|&(f, t)| a >= f && a + len as u32 <= t + 1))
    }
}

/// Packages found in `dirs` (each a folder of package folders). Broken packages are returned
/// as errors so the app can log them without refusing to start.
pub fn discover(dirs: &[PathBuf]) -> Vec<(PathBuf, Result<Package>)> {
    let mut out = Vec::new();
    for root in dirs {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        let mut entries: Vec<_> = rd.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.join("manifest.json").is_file()).collect();
        entries.sort();
        out.extend(entries.into_iter().map(|p| {
            let r = Package::load(&p);
            (p, r)
        }));
    }
    out
}
