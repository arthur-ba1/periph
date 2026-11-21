//! Minimal HID report descriptor parser: enough to list report IDs, their
//! direction (input/output/feature) and size, plus top-level collections.

use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Input,
    Output,
    Feature,
}

#[derive(Debug, Default)]
pub struct Summary {
    /// (report id, kind) -> total bits
    pub reports: BTreeMap<(u8, Kind), u32>,
    /// top-level collections as (usage page, usage)
    pub collections: Vec<(u32, u32)>,
}

impl Summary {
    /// Report length in bytes as hidapi expects it (payload + 1 byte report id).
    pub fn byte_len(bits: u32) -> u32 {
        bits.div_ceil(8) + 1
    }

    pub fn to_text(&self) -> String {
        let mut s = String::new();
        for (page, usage) in &self.collections {
            let _ = writeln!(s, "  collection: usage_page=0x{page:04X} usage=0x{usage:04X}");
        }
        for ((id, kind), bits) in &self.reports {
            let _ = writeln!(
                s,
                "  report id=0x{id:02X} {kind:?}: {bits} bits -> {} bytes incl. id",
                Self::byte_len(*bits)
            );
        }
        s
    }
}

pub fn parse(desc: &[u8]) -> Summary {
    let mut out = Summary::default();
    let (mut usage_page, mut report_size, mut report_count, mut report_id) = (0u32, 0u32, 0u32, 0u8);
    let mut usages: Vec<u32> = Vec::new();
    let mut depth = 0usize;
    let mut i = 0usize;

    while i < desc.len() {
        let prefix = desc[i];
        if prefix == 0xFE {
            // long item: skip
            let len = *desc.get(i + 1).unwrap_or(&0) as usize;
            i += 3 + len;
            continue;
        }
        let size = match prefix & 0x03 {
            3 => 4,
            n => n as usize,
        };
        let ty = (prefix >> 2) & 0x03;
        let tag = prefix >> 4;
        let mut val = 0u32;
        for b in 0..size {
            val |= (*desc.get(i + 1 + b).unwrap_or(&0) as u32) << (8 * b);
        }
        i += 1 + size;

        match (ty, tag) {
            // Main items
            (0, 0x8) | (0, 0x9) | (0, 0xB) => {
                let kind = match tag {
                    0x8 => Kind::Input,
                    0x9 => Kind::Output,
                    _ => Kind::Feature,
                };
                *out.reports.entry((report_id, kind)).or_default() += report_size * report_count;
                usages.clear();
            }
            (0, 0xA) => {
                if depth == 0 {
                    let u = usages.first().copied().unwrap_or(0);
                    // extended usage (32-bit) carries its own page
                    let (p, u) = if u > 0xFFFF { (u >> 16, u & 0xFFFF) } else { (usage_page, u) };
                    out.collections.push((p, u));
                }
                depth += 1;
                usages.clear();
            }
            (0, 0xC) => depth = depth.saturating_sub(1),
            // Global items
            (1, 0x0) => usage_page = val,
            (1, 0x7) => report_size = val,
            (1, 0x8) => report_id = val as u8,
            (1, 0x9) => report_count = val,
            // Local items
            (2, 0x0) => usages.push(val),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_feature_report() {
        // Usage Page (0xFF00), Usage (1), Collection (App), Report ID (6),
        // Report Size (8), Report Count (63), Usage (1), Feature (Data,Var,Abs), End
        let d = [
            0x06, 0x00, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x06, 0x75, 0x08, 0x95, 0x3F, 0x09, 0x01,
            0xB1, 0x02, 0xC0,
        ];
        let s = parse(&d);
        assert_eq!(s.collections, vec![(0xFF00, 1)]);
        assert_eq!(s.reports[&(6, Kind::Feature)], 63 * 8);
        assert_eq!(Summary::byte_len(63 * 8), 64);
    }
}
