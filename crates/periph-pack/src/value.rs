//! Bytes <-> JSON values for one setting element or status field, with the validation that
//! decides what may be written.

use crate::spec::{Codec, Field, Mapping, Range};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    U8,
    I8,
    U16Le,
    U16Be,
    Bool,
    Bit(u8),
    Codec(String),
}

impl Ty {
    pub fn of(f: &Field, codecs: &BTreeMap<String, Codec>) -> Result<Ty, String> {
        Ok(match f.ty.as_str() {
            "u8" => Ty::U8,
            "i8" => Ty::I8,
            "u16le" => Ty::U16Le,
            "u16be" => Ty::U16Be,
            "bool" => Ty::Bool,
            "bit" => match f.bit {
                Some(b) if b < 8 => Ty::Bit(b),
                _ => return Err("type \"bit\" needs \"bit\": 0..7".into()),
            },
            c if codecs.contains_key(c) => Ty::Codec(c.to_string()),
            other => return Err(format!("unknown type \"{other}\"")),
        })
    }

    pub fn size(&self, codecs: &BTreeMap<String, Codec>) -> usize {
        match self {
            Ty::U16Le | Ty::U16Be => 2,
            Ty::Codec(c) => codecs[c].size,
            _ => 1,
        }
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Ty::Bool | Ty::Bit(_))
    }

    fn bounds(&self) -> (i64, i64) {
        match self {
            Ty::I8 => (-128, 127),
            Ty::U16Le | Ty::U16Be => (0, 0xFFFF),
            _ => (0, 0xFF),
        }
    }
}

/// Why a value can't be written; the caller turns it into a translated message.
#[derive(Debug, Clone, PartialEq)]
pub enum Reject {
    Invalid,
    ReadOnly,
}

/// A field with its type and value table resolved.
pub struct Resolved<'a> {
    pub field: &'a Field,
    pub ty: Ty,
    pub values: Option<&'a [Mapping]>,
    pub codecs: &'a BTreeMap<String, Codec>,
}

pub fn same(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() < 1e-9,
        _ => a == b,
    }
}

pub fn in_range(r: &Range, x: f64) -> bool {
    const EPS: f64 = 1e-6;
    if x < r.min - EPS || x > r.max + EPS {
        return false;
    }
    r.step.is_none_or(|s| s <= 0.0 || {
        let k = (x - r.min) / s;
        (k - k.round()).abs() < EPS
    })
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

fn round6(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}

impl Resolved<'_> {
    fn raw(&self, b: &[u8]) -> i64 {
        match self.ty {
            Ty::I8 => b[0] as i8 as i64,
            Ty::U16Le => u16::from_le_bytes([b[0], b[1]]) as i64,
            Ty::U16Be => u16::from_be_bytes([b[0], b[1]]) as i64,
            _ => b[0] as i64,
        }
    }

    /// Decode one element (`bytes` is exactly its size).
    pub fn decode(&self, bytes: &[u8]) -> Value {
        if let Ty::Codec(c) = &self.ty {
            return decode_codec(&self.codecs[c], bytes);
        }
        let raw = self.raw(bytes);
        match self.ty {
            Ty::Bool => return Value::Bool(self.field.equals.map_or(raw != 0, |e| raw == e as i64) != self.field.invert),
            Ty::Bit(b) => return Value::Bool(((raw >> b) & 1 == 1) != self.field.invert),
            _ => {}
        }
        if let Some(values) = self.values {
            return values.iter().find(|m| m.raw as i64 == raw).map_or(Value::Null, |m| m.value.clone());
        }
        match self.field.linear {
            Some(l) if l.scale.fract() == 0.0 && l.offset.fract() == 0.0 => json!(raw * l.scale as i64 + l.offset as i64),
            Some(l) => json!(round6(raw as f64 * l.scale + l.offset)),
            None => json!(raw),
        }
    }

    /// Encode one element, patching `cur` (the element's current bytes) in place.
    pub fn encode(&self, v: &Value, cur: &mut [u8]) -> Result<(), Reject> {
        if let Ty::Codec(c) = &self.ty {
            let bytes = encode_codec(&self.codecs[c], v)?;
            cur.copy_from_slice(&bytes);
            return Ok(());
        }
        if self.ty.is_bool() {
            let on = v.as_bool().ok_or(Reject::Invalid)? != self.field.invert;
            match self.ty {
                Ty::Bit(b) => cur[0] = if on { cur[0] | (1 << b) } else { cur[0] & !(1 << b) },
                _ => match (self.field.equals, on) {
                    (Some(e), true) => cur[0] = u8::try_from(e).map_err(|_| Reject::Invalid)?,
                    (None, true) => cur[0] = 1,
                    (Some(_), false) => return Err(Reject::Invalid), // "off" has no known raw value
                    (None, false) => cur[0] = 0,
                },
            }
            return Ok(());
        }
        let raw: i64 = if let Some(values) = self.values {
            values.iter().find(|m| same(&m.value, v)).ok_or(Reject::Invalid)?.raw as i64
        } else {
            // Plain numbers must be bounded by the package: no range, no writes.
            let range = self.field.range.as_ref().ok_or(Reject::ReadOnly)?;
            let x = v.as_f64().ok_or(Reject::Invalid)?;
            if !in_range(range, x) {
                return Err(Reject::Invalid);
            }
            let r = match self.field.linear {
                Some(l) => (x - l.offset) / l.scale,
                None => x,
            };
            if (r - r.round()).abs() > 1e-6 {
                return Err(Reject::Invalid);
            }
            r.round() as i64
        };
        let (lo, hi) = self.ty.bounds();
        if raw < lo || raw > hi {
            return Err(Reject::Invalid);
        }
        match self.ty {
            Ty::U16Le => cur.copy_from_slice(&(raw as u16).to_le_bytes()),
            Ty::U16Be => cur.copy_from_slice(&(raw as u16).to_be_bytes()),
            _ => cur[0] = raw as u8,
        }
        Ok(())
    }
}

pub fn decode_codec(c: &Codec, bytes: &[u8]) -> Value {
    for v in &c.variants {
        if v.bytes.fixed_len() != bytes.len() {
            continue;
        }
        if let Some((vars, _)) = v.bytes.matches(bytes) {
            return match vars.iter().find(|(n, _)| n == "value") {
                Some((_, x)) => json!({ "kind": v.kind, "value": x }),
                None => json!({ "kind": v.kind }),
            };
        }
    }
    json!({ "kind": "raw", "bytes": hex(bytes) })
}

pub fn encode_codec(c: &Codec, v: &Value) -> Result<Vec<u8>, Reject> {
    let kind = v.get("kind").and_then(Value::as_str).ok_or(Reject::Invalid)?;
    let var = c.variants.iter().find(|x| x.kind == kind).ok_or(Reject::ReadOnly)?; // "raw" and unknown kinds
    if var.read_only {
        return Err(Reject::ReadOnly);
    }
    let takes_value = var.bytes.vars().any(|n| n == "value");
    let value = v.get("value").filter(|x| !x.is_null());
    let x = match (takes_value, value) {
        (true, Some(x)) => {
            let x = x.as_i64().ok_or(Reject::Invalid)?;
            let ok = match (&var.values, &var.range) {
                (Some(vals), _) => vals.contains(&x),
                (None, Some(r)) => in_range(r, x as f64),
                (None, None) => false, // a free value is never written
            };
            if !ok {
                return Err(Reject::Invalid);
            }
            Some(u32::try_from(x).map_err(|_| Reject::Invalid)?)
        }
        (false, None) => None,
        _ => return Err(Reject::Invalid),
    };
    var.bytes.render(|n| if n == "value" { x } else { None }, &[]).map_err(|_| Reject::Invalid)
}
