//! Feature reports the virtual controller answers for games / the PlayStation SDK.
//!
//! The responses are **captured from the user's own physical controller** with
//! `vt2-probe --vid 054c --ds-features` (GET_FEATURE only) and loaded by the driver at runtime;
//! they are not hardcoded here (they contain unit-specific data such as the MAC address).
//! Reports outside the allowlist below are never fabricated: the virtual device either declines
//! them or forwards the physical controller's answer.

/// Report ids the virtual device may answer, with their exact lengths (including the id byte).
pub const ALLOWED: &[(u8, usize)] = &[
    (CALIBRATION_ID, CALIBRATION_LEN),
    (MAC_ADDRESS_ID, MAC_ADDRESS_LEN),
    (FIRMWARE_INFO_ID, FIRMWARE_INFO_LEN),
    (BT_PATCH_INFO_ID, BT_PATCH_INFO_LEN),
];

pub const CALIBRATION_ID: u8 = 0x05;
pub const CALIBRATION_LEN: usize = 41;
pub const MAC_ADDRESS_ID: u8 = 0x09;
pub const MAC_ADDRESS_LEN: usize = 20;
pub const FIRMWARE_INFO_ID: u8 = 0x20;
pub const FIRMWARE_INFO_LEN: usize = 64;
pub const BT_PATCH_INFO_ID: u8 = 0x22;
pub const BT_PATCH_INFO_LEN: usize = 64;

/// One captured response: `bytes[0]` is the report id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureResponse {
    pub bytes: Vec<u8>,
}

impl FeatureResponse {
    pub fn new(id: u8, bytes: &[u8]) -> Result<Self, &'static str> {
        let len = ALLOWED.iter().find(|(rid, _)| *rid == id).map(|(_, l)| *l).ok_or("feature report not in allowlist")?;
        if bytes.len() != len {
            return Err("feature response has wrong length");
        }
        if bytes[0] != id {
            return Err("feature response id mismatch");
        }
        Ok(Self { bytes: bytes.to_vec() })
    }

    pub fn id(&self) -> u8 {
        self.bytes[0]
    }
}

/// The set of responses the virtual device serves, keyed by report id.
#[derive(Debug, Clone, Default)]
pub struct FeatureSet {
    entries: std::collections::BTreeMap<u8, FeatureResponse>,
}

impl FeatureSet {
    pub fn insert(&mut self, response: FeatureResponse) {
        self.entries.insert(response.id(), response);
    }

    pub fn get(&self, id: u8) -> Option<&[u8]> {
        self.entries.get(&id).map(|r| r.bytes.as_slice())
    }

    pub fn is_complete(&self) -> bool {
        ALLOWED.iter().all(|(id, _)| self.entries.contains_key(id))
    }

    /// Parse the output of `vt2-probe --ds-features` (`feature 0x05 (41 bytes): 05 00 ...`).
    pub fn from_probe_text(text: &str) -> Result<Self, &'static str> {
        let mut set = Self::default();
        for line in text.lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("feature 0x") else { continue };
            let id = u8::from_str_radix(&rest[..2], 16).map_err(|_| "bad feature id")?;
            let Some((_, hex)) = line.split_once(": ") else { continue };
            let bytes: Vec<u8> = hex
                .split_whitespace()
                .map(|b| u8::from_str_radix(b, 16).map_err(|_| "bad hex byte"))
                .collect::<Result<_, _>>()?;
            if let Ok(resp) = FeatureResponse::new(id, &bytes) {
                set.insert(resp);
            }
        }
        if set.entries.is_empty() {
            return Err("no feature reports found in probe text");
        }
        Ok(set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: u8, len: usize) -> Vec<u8> {
        let mut v = vec![0xAB; len];
        v[0] = id;
        v
    }

    #[test]
    fn allowlist_enforced() {
        assert!(FeatureResponse::new(0x05, &sample(0x05, 41)).is_ok());
        assert!(FeatureResponse::new(0xF0, &sample(0xF0, 64)).is_err()); // factory/test: never
        assert!(FeatureResponse::new(0x05, &sample(0x05, 40)).is_err());
        assert!(FeatureResponse::new(0x20, &sample(0x21, 64)).is_err());
    }

    #[test]
    fn parse_probe_capture() {
        // Real GET_FEATURE capture (see docs/dualsense-protocol.md), with the MAC address replaced by a placeholder.
        let text = "\
## \\\\?\\HID#VID_054C...  serial=\"\"
  feature 0x05 (41 bytes): 05 00 00 EF FF 0B 00 AE 22 53 DD 8B 22 48 DD C5 22 5A DD 1C 02 1C 02 0F 20 E8 DF BE 1F 88 DF E9 1F 28 E0 08 00 00 00 00 00
  feature 0x09 (20 bytes): 09 02 11 22 33 44 55 08 25 00 00 00 00 00 00 00 00 00 00 00
  feature 0x20 (64 bytes): 20 4A 75 6C 20 20 34 20 32 30 32 35 31 30 3A 31 30 3A 33 32 03 00 04 00 18 07 00 00 2A 00 10 01 00 98 04 00 00 00 00 00 00 00 00 00 30 06 00 00 3C 00 01 00 0A 00 02 00 06 00 00 00 00 00 00 00
  feature 0x22 (64 bytes): 22 03 00 18 07 00 00 2A 00 10 01 00 00 00 00 00 00 02 11 22 33 44 55 0A 00 02 00 06 00 00 00 19 00 00 00 21 28 01 00 04 00 02 02 00 00 91 AD 32 03 16 30 0B 00 3C 00 01 00 58 05 00 00 00 00 00
";
        let set = FeatureSet::from_probe_text(text).unwrap();
        assert!(set.is_complete());
        let cal = set.get(0x05).unwrap();
        assert_eq!(cal.len(), 41);
        assert_eq!(&cal[1..5], &[0x00, 0x00, 0xEF, 0xFF]);
        assert_eq!(set.get(0x09).unwrap()[1], 0x02); // first MAC byte (placeholder MAC)
        assert_eq!(set.get(0x20).unwrap()[1], b'J'); // "Jul  4 2025..." firmware build string
        assert_eq!(set.get(0xF0), None);
    }

    #[test]
    fn incomplete_set_is_reported() {
        let mut set = FeatureSet::default();
        set.insert(FeatureResponse::new(0x05, &sample(0x05, 41)).unwrap());
        assert!(!set.is_complete());
    }
}
