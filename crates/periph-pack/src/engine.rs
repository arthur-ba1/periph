//! Runs a package against a [`Port`]: reads the config, applies commands, parses status.
//!
//! Two transports (see `docs/device-packages.md`):
//! - `registers`: a memory map read and written with request/answer reports (VT2 style);
//! - `reports`: one templated request per setting, state fetched with `reads`.

use crate::checksum;
use crate::package::Package;
use crate::port::Port;
use crate::spec::{Answer, Match, Range, Read, Registers, ReportKind, Reports, Setting, Transport};
use crate::template::Template;
use crate::value::{Reject, Resolved};
use anyhow::{anyhow, bail, Result};
use periph_core::{t, tf, Absent};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Commands from the UI and the tray.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Cmd {
    Refresh,
    Set {
        id: String,
        #[serde(default)]
        index: Option<usize>,
        value: Value,
    },
}

/// What a session remembers between commands (`reports` transport).
#[derive(Debug, Default)]
pub struct State {
    /// Last answer of each read, report id stripped.
    pub answers: BTreeMap<String, Vec<u8>>,
    /// Last value written to settings that can not be read back.
    pub written: BTreeMap<String, Value>,
}

pub struct Engine<'a> {
    pub pkg: &'a Package,
    pub port: &'a mut dyn Port,
    pub state: &'a mut State,
}

/// An answer without its report id (hidapi includes it for numbered reports, not always).
pub fn payload(raw: &[u8], report_id: u8) -> &[u8] {
    if report_id != 0 && raw.first() == Some(&report_id) {
        &raw[1..]
    } else {
        raw
    }
}

pub fn matches(rules: &[Match], p: &[u8]) -> bool {
    rules.iter().all(|m| p.get(m.at).is_some_and(|b| b & m.mask.unwrap_or(0xFF) == m.equals))
}

fn invalid(id: &str) -> anyhow::Error {
    anyhow!(tf("packages.errors.invalidValue", &[("id", &id)]))
}

fn encode(r: &Resolved, id: &str, v: &Value, cur: &mut [u8]) -> Result<()> {
    r.encode(v, cur).map_err(|e| match e {
        Reject::ReadOnly => anyhow!(tf("packages.errors.readOnly", &[("id", &id)])),
        Reject::Invalid => invalid(id),
    })
}

fn var(name: &str, addr: u32, len: usize) -> Option<u32> {
    match name {
        "addr" => Some(addr),
        "len" => Some(len as u32),
        _ => None,
    }
}

impl<'a> Engine<'a> {
    pub fn new(pkg: &'a Package, port: &'a mut dyn Port, state: &'a mut State) -> Self {
        Engine { pkg, port, state }
    }

    pub fn read_config(&mut self) -> Result<Value> {
        if let Transport::Reports(_) = &self.pkg.device.transport {
            self.fetch_all()?;
        }
        let mut out = Map::new();
        for s in &self.pkg.device.settings {
            let v = match &self.pkg.device.transport {
                Transport::Registers(_) => self.read_register_setting(s)?,
                Transport::Reports(_) => self.report_setting(s)?,
            };
            out.insert(s.id.clone(), v);
        }
        Ok(Value::Object(out))
    }

    pub fn apply(&mut self, cmd: Cmd) -> Result<()> {
        let Cmd::Set { id, index, value } = cmd else { return Ok(()) };
        let pkg = self.pkg;
        let s = pkg.setting(&id).ok_or_else(|| anyhow!(tf("packages.errors.unknownSetting", &[("id", &id)])))?;
        let writable = match &pkg.device.transport {
            Transport::Registers(_) => !s.read_only,
            Transport::Reports(_) => !s.read_only && !s.send.is_empty(),
        };
        if !writable {
            bail!(tf("packages.errors.readOnly", &[("id", &id)]));
        }
        let index = match (s.field.count, index) {
            (Some(n), Some(i)) if i < n => i,
            (None, None) => 0,
            _ => return Err(invalid(&id)),
        };
        // An index into another array setting is bounded by that array.
        let mut field = s.field.clone();
        if let Some(of) = &s.index_of {
            let n = pkg.setting(of).and_then(|o| o.field.count).unwrap_or(0);
            if field.range.is_none() && n > 0 {
                field.range = Some(Range { min: 0.0, max: (n - 1) as f64, step: Some(1.0) });
            }
        }
        let r = pkg.resolve(&field)?;
        match &pkg.device.transport {
            Transport::Registers(_) => self.apply_registers(s, &r, index, &value),
            Transport::Reports(_) => self.apply_reports(s, &r, index, &value),
        }
    }

    // ---- registers ----

    fn regs(&self) -> &'a Registers {
        match &self.pkg.device.transport {
            Transport::Registers(r) => r,
            Transport::Reports(_) => unreachable!("registers transport"),
        }
    }

    fn transact(&mut self, req: Vec<u8>) -> Result<Vec<u8>> {
        let spec = self.regs();
        let r = &spec.response;
        let mut report = req;
        report.resize(spec.report_length.max(report.len()), 0);
        self.port.write(&report)?;
        let deadline = Instant::now() + Duration::from_millis(r.timeout_ms);
        loop {
            let raw = self.port.input_report(r.report_id, spec.report_length.max(r.data + 1))?;
            let buf = payload(&raw, r.report_id);
            let st = *buf.get(r.status).ok_or_else(|| anyhow!(t("packages.errors.badResponse")))?;
            let waiting = r.busy.contains(&st) || r.pending.contains(&st);
            if r.ready.contains(&st) {
                return Ok(buf.get(r.data..).unwrap_or_default().to_vec());
            } else if waiting && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(2));
            } else if r.pending.contains(&st) {
                let code = self.pkg.pending_code.unwrap_or("notFound");
                return Err(Absent::new(code, t("packages.errors.noAnswer")).into());
            } else if r.busy.contains(&st) {
                bail!(t("packages.errors.busy"));
            } else {
                bail!("{}: {st:02X}", t("packages.errors.badResponse"));
            }
        }
    }

    fn readable(&self, addr: u32, len: usize) -> bool {
        let spec = self.regs();
        spec.readable.is_empty() || spec.readable.iter().any(|s| addr >= s.from && addr + len as u32 <= s.to + 1)
    }

    pub fn read(&mut self, addr: u32, len: usize) -> Result<Vec<u8>> {
        if !self.readable(addr, len) {
            bail!("{} 0x{addr:04X}", t("packages.errors.notReadable"));
        }
        let max = self.regs().max_chunk;
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let a = addr + out.len() as u32;
            let n = (len - out.len()).min(max);
            let req = self.regs().read.render(|v| var(v, a, n), &[]).map_err(|e| anyhow!(e))?;
            let data = self.transact(req)?;
            if data.len() < n {
                bail!(t("packages.errors.badResponse"));
            }
            out.extend_from_slice(&data[..n]);
        }
        Ok(out)
    }

    pub fn write(&mut self, addr: u32, data: &[u8]) -> Result<()> {
        let max = self.regs().max_chunk;
        for (i, chunk) in data.chunks(max).enumerate() {
            let a = addr + (i * max) as u32;
            let req = self.regs().write.render(|v| var(v, a, chunk.len()), chunk).map_err(|e| anyhow!(e))?;
            self.transact(req)?;
        }
        Ok(())
    }

    fn read_register_setting(&mut self, s: &Setting) -> Result<Value> {
        let r = self.pkg.resolve(&s.field)?;
        let size = r.ty.size(&self.pkg.device.codecs);
        let count = s.field.count.unwrap_or(1);
        let bytes = self.read(s.at[0], size * count)?;
        let items: Vec<Value> = bytes.chunks(size).map(|b| r.decode(b)).collect();
        Ok(if s.field.count.is_some() { Value::Array(items) } else { items.into_iter().next().unwrap_or(Value::Null) })
    }

    fn apply_registers(&mut self, s: &Setting, r: &Resolved, index: usize, value: &Value) -> Result<()> {
        let size = r.ty.size(&self.pkg.device.codecs);
        let elems: Vec<u32> = s.at.iter().map(|a| a + (index * size) as u32).collect();
        let default_len = self.regs().write_block.max(size);
        let blocks: Vec<(u32, usize)> = match &s.write {
            Some(b) => b.iter().map(|b| (b.at, b.len)).collect(),
            None => elems.iter().map(|&a| (a, default_len)).collect(),
        };
        // Read every block first, patch, then write them all (like the official apps do).
        let mut bufs = Vec::new();
        for &(at, len) in &blocks {
            bufs.push(self.read(at, len)?);
        }
        let mut patched = vec![false; elems.len()];
        for ((at, len), buf) in blocks.iter().zip(&mut bufs) {
            for (k, &e) in elems.iter().enumerate() {
                if e >= *at && e + size as u32 <= at + *len as u32 {
                    let off = (e - at) as usize;
                    encode(r, &s.id, value, &mut buf[off..off + size])?;
                    patched[k] = true;
                }
            }
        }
        if patched.contains(&false) {
            bail!("{}: write blocks do not cover every address", s.id);
        }
        for ((at, _), buf) in blocks.iter().zip(&bufs) {
            self.write(*at, buf)?;
        }
        Ok(())
    }

    // ---- reports ----

    fn reports(&self) -> &'a Reports {
        match &self.pkg.device.transport {
            Transport::Reports(r) => r,
            Transport::Registers(_) => unreachable!("reports transport"),
        }
    }

    /// Render, pad, checksum and send one request.
    pub fn send(&mut self, tpl: &Template, index: Option<u32>, value: &[u8]) -> Result<()> {
        let spec = self.reports();
        let report = request(spec, tpl, index, value)?;
        match spec.send {
            ReportKind::Feature => self.port.send_feature(&report)?,
            _ => self.port.write(&report)?,
        }
        if spec.delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(spec.delay_ms));
        }
        Ok(())
    }

    fn answer(&mut self, a: &Answer) -> Result<Vec<u8>> {
        let deadline = Instant::now() + Duration::from_millis(a.timeout_ms);
        let len = a.length.unwrap_or(self.reports().report_length);
        loop {
            let raw = match a.from {
                ReportKind::Feature => Some(self.port.get_feature(a.report_id, len)?),
                _ => self.port.read(50)?,
            };
            if let Some(raw) = raw {
                let ok_id = a.report_id == 0 || raw.first() == Some(&a.report_id);
                let p = payload(&raw, a.report_id);
                if ok_id && matches(&a.r#match, p) {
                    return Ok(p.to_vec());
                }
            }
            if Instant::now() >= deadline {
                bail!(t("packages.errors.noAnswer"));
            }
            if a.from == ReportKind::Feature {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    pub fn fetch(&mut self, read: &Read) -> Result<Vec<u8>> {
        self.send(&read.request, None, &[])?;
        self.answer(&read.response)
    }

    fn fetch_all(&mut self) -> Result<()> {
        self.state.answers.clear();
        for read in &self.pkg.device.reads {
            let p = self.fetch(read)?;
            self.state.answers.insert(read.id.clone(), p);
        }
        Ok(())
    }

    fn report_setting(&self, s: &Setting) -> Result<Value> {
        let Some(from) = &s.from else {
            let v = self.state.written.get(&s.id).or(s.default.as_ref()).cloned().unwrap_or(Value::Null);
            return Ok(v);
        };
        let r = self.pkg.resolve(&s.field)?;
        let size = r.ty.size(&self.pkg.device.codecs);
        let count = s.field.count.unwrap_or(1);
        let at = s.at[0] as usize;
        let p = self.state.answers.get(from).map(Vec::as_slice).unwrap_or_default();
        let bytes = p.get(at..at + size * count).ok_or_else(|| anyhow!("{}: {} ({})", t("packages.errors.badResponse"), s.id, from))?;
        let items: Vec<Value> = bytes.chunks(size).map(|b| r.decode(b)).collect();
        Ok(if s.field.count.is_some() { Value::Array(items) } else { items.into_iter().next().unwrap_or(Value::Null) })
    }

    fn apply_reports(&mut self, s: &Setting, r: &Resolved, index: usize, value: &Value) -> Result<()> {
        let size = r.ty.size(&self.pkg.device.codecs);
        // Current bytes (bit fields keep their neighbours), or zeros when it can't be read.
        let mut cur = match &s.from {
            Some(from) => {
                let at = s.at[0] as usize + index * size;
                self.state.answers.get(from).and_then(|p| p.get(at..at + size)).map(<[u8]>::to_vec).unwrap_or_else(|| vec![0; size])
            }
            None => vec![0; size],
        };
        encode(r, &s.id, value, &mut cur)?;
        let idx = s.field.count.map(|_| index as u32);
        for tpl in &s.send {
            self.send(tpl, idx, &cur)?;
        }
        for tpl in &self.reports().after_write {
            self.send(tpl, None, &[])?;
        }
        if s.from.is_none() {
            let v = match s.field.count {
                Some(n) => {
                    let mut arr = self.state.written.get(&s.id).and_then(Value::as_array).cloned().unwrap_or_else(|| vec![Value::Null; n]);
                    arr[index] = value.clone();
                    Value::Array(arr)
                }
                None => value.clone(),
            };
            self.state.written.insert(s.id.clone(), v);
        }
        Ok(())
    }
}

/// The bytes of one request: template filled, zero-padded, checksum written.
pub fn request(spec: &Reports, tpl: &Template, index: Option<u32>, value: &[u8]) -> Result<Vec<u8>> {
    let mut report = tpl.render_value(index, value).map_err(|e| anyhow!(e))?;
    if report.len() > spec.report_length {
        bail!("request longer than reportLength ({} > {})", report.len(), spec.report_length);
    }
    report.resize(spec.report_length, 0);
    if let Some(c) = &spec.checksum {
        checksum::apply(c, &mut report).map_err(|e| anyhow!(e))?;
    }
    Ok(report)
}

/// Parse a report the device sent on its own; `None` if it is not the status report.
pub fn parse_status(pkg: &Package, raw: &[u8]) -> Option<Value> {
    let st = pkg.device.status.as_ref()?;
    if st.report_id != 0 && raw.first() != Some(&st.report_id) {
        return None;
    }
    let buf = payload(raw, st.report_id);
    if !matches(&st.r#match, buf) {
        return None;
    }
    let mut out = Map::new();
    for f in &st.fields {
        let r = pkg.resolve(&f.field).ok()?;
        let size = r.ty.size(&pkg.device.codecs);
        out.insert(f.id.clone(), r.decode(buf.get(f.at..f.at + size)?));
    }
    Some(Value::Object(out))
}
