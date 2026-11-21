//! Checksums written into requests (`transport.checksum`).

use crate::spec::{parse_hex, Checksum};

/// Resolve a position that may count from the end (-1 = last byte).
fn pos(p: i64, len: usize) -> Option<usize> {
    let v = if p < 0 { len as i64 + p } else { p };
    (0..len as i64).contains(&v).then_some(v as usize)
}

pub fn size(kind: &str) -> Option<usize> {
    Some(match kind {
        "sum8" | "xor8" | "crc8" => 1,
        "crc16ccitt" => 2,
        "crc32" => 4,
        _ => return None,
    })
}

fn crc8(data: &[u8]) -> u32 {
    let mut c = 0u8;
    for &b in data {
        c ^= b;
        for _ in 0..8 {
            c = if c & 0x80 != 0 { (c << 1) ^ 0x07 } else { c << 1 };
        }
    }
    c as u32
}

fn crc16ccitt(data: &[u8]) -> u32 {
    let mut c = 0xFFFFu16;
    for &b in data {
        c ^= (b as u16) << 8;
        for _ in 0..8 {
            c = if c & 0x8000 != 0 { (c << 1) ^ 0x1021 } else { c << 1 };
        }
    }
    c as u32
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

/// Check the spec against a report length; `Err` explains what is wrong.
pub fn validate(c: &Checksum, len: usize) -> Result<(), String> {
    let n = size(&c.kind).ok_or_else(|| format!("unknown checksum type \"{}\" (sum8, xor8, crc8, crc16ccitt, crc32)", c.kind))?;
    match (pos(c.from, len), pos(c.to, len), pos(c.at, len)) {
        (Some(f), Some(t), Some(a)) if f <= t && a + n <= len && (a > t || a + n <= f) => {}
        _ => return Err(format!("checksum positions (from {}, to {}, at {}) do not fit a {len}-byte report", c.from, c.to, c.at)),
    }
    if let Some(p) = &c.prefix {
        parse_hex(p)?;
    }
    Ok(())
}

/// Write the checksum into `report` (already padded to its final length).
pub fn apply(c: &Checksum, report: &mut [u8]) -> Result<(), String> {
    validate(c, report.len())?;
    let len = report.len();
    let (from, to, at) = (pos(c.from, len).unwrap(), pos(c.to, len).unwrap(), pos(c.at, len).unwrap());
    let mut data = c.prefix.as_deref().map(parse_hex).transpose()?.unwrap_or_default();
    data.extend_from_slice(&report[from..=to]);
    let v = match c.kind.as_str() {
        "sum8" => {
            let s = data.iter().fold(0u8, |a, b| a.wrapping_add(*b));
            (if c.negate { s.wrapping_neg() } else { s }) as u32
        }
        "xor8" => data.iter().fold(0u8, |a, b| a ^ b) as u32,
        "crc8" => crc8(&data),
        "crc16ccitt" => crc16ccitt(&data),
        _ => crc32(&data),
    };
    let n = size(&c.kind).unwrap();
    let bytes = v.to_le_bytes();
    for i in 0..n {
        report[at + i] = if c.big_endian { bytes[n - 1 - i] } else { bytes[i] };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ck(kind: &str) -> Checksum {
        Checksum { kind: kind.into(), from: 0, to: 8, at: -4, negate: false, big_endian: false, prefix: None }
    }

    #[test]
    fn known_vectors() {
        // check values of the standard algorithms over "123456789"
        let mut r = b"123456789\0\0\0\0".to_vec();
        apply(&ck("crc32"), &mut r).unwrap();
        assert_eq!(u32::from_le_bytes(r[9..13].try_into().unwrap()), 0xCBF4_3926);
        let mut r = b"123456789\0\0\0\0".to_vec();
        apply(&ck("crc16ccitt"), &mut r).unwrap();
        assert_eq!(u16::from_le_bytes(r[9..11].try_into().unwrap()), 0x29B1);
        let mut r = b"123456789\0\0\0\0".to_vec();
        apply(&ck("crc8"), &mut r).unwrap();
        assert_eq!(r[9], 0xF4);
        let mut r = vec![1, 2, 3, 0];
        let neg = Checksum { kind: "sum8".into(), from: 0, to: 2, at: -1, negate: true, big_endian: false, prefix: None };
        apply(&neg, &mut r).unwrap();
        assert_eq!(r[3], 0xFA);
        assert!(validate(&ck("crc32"), 5).is_err());
    }
}
