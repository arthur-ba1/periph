//! The JSON files of a package, as written by its author. See `docs/device-packages.md`.

use crate::template::Template;
use serde::{de, Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeMap;

/// A number written as a JSON number or as a string ("0x24AE", "36").
pub fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim();
    let (neg, s) = s.strip_prefix('-').map_or((false, s), |r| (true, r));
    let v = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(h) => i64::from_str_radix(h, 16).ok()?,
        None => s.parse().ok()?,
    };
    Some(if neg { -v } else { v })
}

fn int<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    match Value::deserialize(d)? {
        Value::Number(n) => n.as_i64().ok_or_else(|| de::Error::custom(format!("{n} is not an integer"))),
        Value::String(s) => parse_int(&s).ok_or_else(|| de::Error::custom(format!("\"{s}\" is not a number"))),
        v => Err(de::Error::custom(format!("expected a number, got {v}"))),
    }
}

fn num<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    match Value::deserialize(d)? {
        Value::Number(n) => Ok(n.as_f64().unwrap_or_default()),
        Value::String(s) => parse_int(&s).map(|v| v as f64).ok_or_else(|| de::Error::custom(format!("\"{s}\" is not a number"))),
        v => Err(de::Error::custom(format!("expected a number, got {v}"))),
    }
}

fn ranged<'de, D: Deserializer<'de>, T: TryFrom<i64>>(d: D) -> Result<T, D::Error> {
    let v = int(d)?;
    T::try_from(v).map_err(|_| de::Error::custom(format!("{v} is out of range")))
}

fn opt_ranged<'de, D: Deserializer<'de>, T: TryFrom<i64>>(d: D) -> Result<Option<T>, D::Error> {
    ranged(d).map(Some)
}

fn ranged_vec<'de, D: Deserializer<'de>, T: TryFrom<i64>>(d: D) -> Result<Vec<T>, D::Error> {
    #[derive(Deserialize)]
    struct W(#[serde(deserialize_with = "int")] i64);
    Vec::<W>::deserialize(d)?
        .into_iter()
        .map(|W(v)| T::try_from(v).map_err(|_| de::Error::custom(format!("{v} is out of range"))))
        .collect()
}

/// One address or several that always hold the same value.
fn addrs<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u32>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum A {
        Many(Vec<Value>),
        One(Value),
    }
    let one = |v: Value| -> Result<u32, D::Error> {
        let n = match &v {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => parse_int(s),
            _ => None,
        };
        n.and_then(|n| u32::try_from(n).ok()).ok_or_else(|| de::Error::custom(format!("bad address {v}")))
    };
    match A::deserialize(d)? {
        A::Many(v) => v.into_iter().map(one).collect(),
        A::One(v) => Ok(vec![one(v)?]),
    }
}

fn templates<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Template>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum T {
        Many(Vec<Template>),
        One(Template),
    }
    Ok(match T::deserialize(d)? {
        T::Many(v) => v,
        T::One(t) => vec![t],
    })
}

/// "06 A5 A4" -> [6, 0xA5, 0xA4]
pub fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    s.split_whitespace().map(|b| u8::from_str_radix(b, 16).map_err(|_| format!("bad hex byte \"{b}\""))).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Mouse,
    Keyboard,
    Headset,
    Controller,
    Other,
}

impl From<Kind> for periph_core::Kind {
    fn from(k: Kind) -> Self {
        match k {
            Kind::Mouse => periph_core::Kind::Mouse,
            Kind::Keyboard => periph_core::Kind::Keyboard,
            Kind::Headset => periph_core::Kind::Headset,
            Kind::Controller => periph_core::Kind::Controller,
            Kind::Other => periph_core::Kind::Other,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub format: u32,
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Built-in driver this package supersedes (not started while the package is installed).
    #[serde(default)]
    pub replaces: Option<String>,
    /// Tried in order; the first one present is opened.
    pub products: Vec<Product>,
    pub interfaces: BTreeMap<String, Interface>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Product {
    #[serde(deserialize_with = "ranged")]
    pub vid: u16,
    #[serde(deserialize_with = "ranged")]
    pub pid: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Interface {
    #[serde(deserialize_with = "ranged")]
    pub usage_page: u16,
    #[serde(deserialize_with = "ranged")]
    pub usage: u16,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Device {
    pub transport: Transport,
    #[serde(default)]
    pub codecs: BTreeMap<String, Codec>,
    pub settings: Vec<Setting>,
    /// `reports` transport: the requests that fetch the device's state.
    #[serde(default)]
    pub reads: Vec<Read>,
    #[serde(default)]
    pub status: Option<Status>,
    #[serde(default)]
    pub tray: Option<Tray>,
    #[serde(default)]
    pub pages: Vec<Page>,
    #[serde(default)]
    pub examples: Vec<Example>,
    /// Fake state for the UI demo mode; the engine ignores it.
    #[serde(default)]
    pub demo: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Transport {
    Registers(Registers),
    Reports(Reports),
}

impl Transport {
    pub fn interface(&self) -> &str {
        match self {
            Transport::Registers(r) => &r.interface,
            Transport::Reports(r) => &r.interface,
        }
    }
}

/// Each setting is written with its own report, filled from a template; the state is read with
/// the requests in `Device::reads`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reports {
    pub interface: String,
    /// How requests go out: `output` (default) or `feature` reports.
    #[serde(default)]
    pub send: ReportKind,
    /// Requests are zero-padded to this length (report id included).
    pub report_length: usize,
    #[serde(default)]
    pub checksum: Option<Checksum>,
    /// Sent after every write, e.g. "save to flash" (must be observed from the official app).
    #[serde(default)]
    pub after_write: Vec<Template>,
    /// Pause between two requests.
    #[serde(default)]
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReportKind {
    #[default]
    Output,
    Feature,
    Input,
}

/// A checksum written into every request. Positions count the report id as byte 0; negative
/// positions count from the end (-1 = last byte).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Checksum {
    /// sum8, xor8, crc8, crc16ccitt, crc32
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default = "one")]
    pub from: i64,
    #[serde(default = "minus_two")]
    pub to: i64,
    #[serde(default = "minus_one")]
    pub at: i64,
    /// sum8 only: write 0x100 - sum (the bytes then add up to zero).
    #[serde(default)]
    pub negate: bool,
    /// crc16/crc32 byte order: little endian unless this is set.
    #[serde(default)]
    pub big_endian: bool,
    /// Bytes fed to the checksum before the report (e.g. "A2" for DualSense Bluetooth).
    #[serde(default)]
    pub prefix: Option<String>,
}

fn one() -> i64 {
    1
}
fn minus_one() -> i64 {
    -1
}
fn minus_two() -> i64 {
    -2
}

/// A request whose answer holds part of the device state (`reports` transport).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Read {
    pub id: String,
    pub request: Template,
    pub response: Answer,
}

/// Where an answer comes from and how to recognize it. Offsets in answers never count the
/// report id (the engine strips it).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Answer {
    /// `feature` (get feature report) or `input` (next matching input report).
    pub from: ReportKind,
    #[serde(deserialize_with = "ranged")]
    pub report_id: u8,
    /// Length to request for a feature report (report id included).
    #[serde(default)]
    pub length: Option<usize>,
    #[serde(default)]
    pub r#match: Vec<Match>,
    #[serde(default = "half")]
    pub timeout_ms: u64,
}

fn half() -> u64 {
    500
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Registers {
    pub interface: String,
    pub report_length: usize,
    pub read: Template,
    pub write: Template,
    pub max_chunk: usize,
    #[serde(default = "four")]
    pub write_block: usize,
    #[serde(default)]
    pub readable: Vec<Span>,
    pub response: Response,
}

fn four() -> usize {
    4
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Span {
    #[serde(deserialize_with = "ranged")]
    pub from: u32,
    #[serde(deserialize_with = "ranged")]
    pub to: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Response {
    #[serde(deserialize_with = "ranged")]
    pub report_id: u8,
    /// Offset of the status byte in the answer, not counting the report id (hidapi on Windows
    /// returns some answers without it; the engine strips it when present).
    pub status: usize,
    /// Offset of the first data byte, not counting the report id.
    pub data: usize,
    #[serde(deserialize_with = "ranged_vec")]
    pub ready: Vec<u8>,
    #[serde(default, deserialize_with = "ranged_vec")]
    pub busy: Vec<u8>,
    #[serde(default, deserialize_with = "ranged_vec")]
    pub pending: Vec<u8>,
    #[serde(default)]
    pub pending_code: Option<String>,
    #[serde(default = "thousand")]
    pub timeout_ms: u64,
}

fn thousand() -> u64 {
    1000
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mapping {
    pub value: Value,
    #[serde(deserialize_with = "ranged")]
    pub raw: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Linear {
    #[serde(deserialize_with = "num")]
    pub scale: f64,
    #[serde(deserialize_with = "num")]
    pub offset: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Range {
    #[serde(deserialize_with = "num")]
    pub min: f64,
    #[serde(deserialize_with = "num")]
    pub max: f64,
    #[serde(default, deserialize_with = "opt_num")]
    pub step: Option<f64>,
}

fn opt_num<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    num(d).map(Some)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    #[serde(deserialize_with = "ranged")]
    pub at: u32,
    pub len: usize,
}

/// How bytes become a value. Shared by settings (register addresses) and status fields
/// (offsets in a report).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub bit: Option<u8>,
    #[serde(default)]
    pub invert: bool,
    /// For `bool`: true when the raw value equals this (default: non-zero).
    #[serde(default, deserialize_with = "opt_ranged")]
    pub equals: Option<u32>,
    #[serde(default)]
    pub count: Option<usize>,
    #[serde(default)]
    pub values: Option<Vec<Mapping>>,
    /// Use the `values` of this setting.
    #[serde(default)]
    pub values_from: Option<String>,
    #[serde(default)]
    pub linear: Option<Linear>,
    #[serde(default)]
    pub range: Option<Range>,
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub id: String,
    /// `registers`: address (or mirrors). `reports`: offset in the answer of `from`.
    #[serde(default, deserialize_with = "addrs")]
    pub at: Vec<u32>,
    #[serde(flatten)]
    pub field: Field,
    /// `registers`: explicit read-modify-write blocks.
    #[serde(default)]
    pub write: Option<Vec<Block>>,
    /// `reports`: the read (by id) whose answer holds this setting.
    #[serde(default)]
    pub from: Option<String>,
    /// `reports`: request(s) that write it. `{value}` is the encoded value, `{index}` the item.
    #[serde(default, deserialize_with = "templates")]
    pub send: Vec<Template>,
    /// `reports`: value shown for a setting that can not be read back (no `from`).
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub index_of: Option<String>,
    #[serde(default)]
    pub count_from: Option<String>,
    #[serde(default)]
    pub control: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Codec {
    pub size: usize,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Variant {
    pub kind: String,
    pub bytes: Template,
    #[serde(default)]
    pub values: Option<Vec<i64>>,
    #[serde(default)]
    pub range: Option<Range>,
    #[serde(default)]
    pub read_only: bool,
    /// UI hint for the value, e.g. "hidKey" (keyboard usage names).
    #[serde(default)]
    pub display: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Status {
    pub interface: String,
    /// Report id of the status report (0 when the device uses none). Offsets do not count it.
    #[serde(deserialize_with = "ranged")]
    pub report_id: u8,
    #[serde(default)]
    pub r#match: Vec<Match>,
    pub fields: Vec<StatusField>,
    #[serde(default = "quarter")]
    pub timeout_ms: u64,
}

fn quarter() -> u64 {
    250
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Match {
    pub at: usize,
    #[serde(default, deserialize_with = "opt_ranged")]
    pub mask: Option<u8>,
    #[serde(deserialize_with = "ranged")]
    pub equals: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusField {
    pub id: String,
    pub at: usize,
    #[serde(flatten)]
    pub field: Field,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tray {
    #[serde(default)]
    pub battery: Option<TrayBattery>,
    #[serde(default)]
    pub header: Option<String>,
    #[serde(default)]
    pub items: Vec<TrayEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrayBattery {
    pub level: String,
    #[serde(default)]
    pub charging: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrayEntry {
    #[serde(default)]
    pub setting: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub refresh: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Page {
    pub id: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub settings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Example {
    #[serde(default)]
    pub note: Option<String>,
    /// Register contents before the command: address -> hex bytes.
    #[serde(default)]
    pub memory: BTreeMap<String, String>,
    /// `reports`: answer of each read before the command: read id -> hex (after the report id).
    #[serde(default)]
    pub responses: BTreeMap<String, String>,
    pub set: Value,
    /// Expected write requests, in order (hex, without the zero padding).
    pub writes: Vec<String>,
}

impl Example {
    pub fn memory(&self) -> Result<Vec<(u32, Vec<u8>)>, String> {
        self.memory
            .iter()
            .map(|(a, b)| {
                let addr = parse_int(a).and_then(|v| u32::try_from(v).ok()).ok_or_else(|| format!("bad address \"{a}\""))?;
                Ok((addr, parse_hex(b)?))
            })
            .collect()
    }
}
