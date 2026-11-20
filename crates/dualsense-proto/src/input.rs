//! USB input report 0x01 (64 bytes): buttons, sticks, triggers, motion, touchpad, battery.
//!
//! Layout follows the real controller (see `docs/dualsense-protocol.md`). Fields whose meaning
//! is not confirmed are kept as reserved bytes and forwarded untouched.

use crate::{INPUT_REPORT_ID, INPUT_REPORT_LEN};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Buttons {
    /// D-pad as the controller encodes it: 0..=7 clockwise from North, 8 = released.
    pub dpad: u8,
    pub square: bool,
    pub cross: bool,
    pub circle: bool,
    pub triangle: bool,
    pub l1: bool,
    pub r1: bool,
    pub l2: bool,
    pub r2: bool,
    pub share: bool,
    pub options: bool,
    pub l3: bool,
    pub r3: bool,
    pub ps: bool,
    pub touchpad: bool,
    pub mute: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputReport {
    pub raw: [u8; INPUT_REPORT_LEN],
}

impl Default for InputReport {
    fn default() -> Self {
        Self::neutral()
    }
}

impl InputReport {
    /// Idle report: sticks centered, nothing pressed, the constants a real pad sends.
    pub fn neutral() -> Self {
        let mut raw = [0u8; INPUT_REPORT_LEN];
        raw[0] = INPUT_REPORT_ID;
        raw[1..5].fill(0x80); // LX LY RX RY
        raw[8] = 8; // dpad released
        raw[32] = 14;
        raw[53] = 42;
        raw[54] = 8;
        Self { raw }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != INPUT_REPORT_LEN {
            return Err("input report must be 64 bytes");
        }
        if bytes[0] != INPUT_REPORT_ID {
            return Err("input report id must be 0x01");
        }
        let mut raw = [0u8; INPUT_REPORT_LEN];
        raw.copy_from_slice(bytes);
        Ok(Self { raw })
    }

    pub fn as_bytes(&self) -> &[u8; INPUT_REPORT_LEN] {
        &self.raw
    }

    pub fn sticks(&self) -> (u8, u8, u8, u8) {
        (self.raw[1], self.raw[2], self.raw[3], self.raw[4])
    }

    pub fn set_sticks(&mut self, lx: u8, ly: u8, rx: u8, ry: u8) {
        self.raw[1..5].copy_from_slice(&[lx, ly, rx, ry]);
    }

    pub fn triggers(&self) -> (u8, u8) {
        (self.raw[5], self.raw[6])
    }

    pub fn set_triggers(&mut self, l2: u8, r2: u8) {
        self.raw[5] = l2;
        self.raw[6] = r2;
    }

    pub fn buttons(&self) -> Buttons {
        let b = &self.raw;
        Buttons {
            dpad: b[8] & 0x0F,
            square: b[8] & 0x10 != 0,
            cross: b[8] & 0x20 != 0,
            circle: b[8] & 0x40 != 0,
            triangle: b[8] & 0x80 != 0,
            l1: b[9] & 0x01 != 0,
            r1: b[9] & 0x02 != 0,
            l2: b[9] & 0x04 != 0,
            r2: b[9] & 0x08 != 0,
            share: b[9] & 0x10 != 0,
            options: b[9] & 0x20 != 0,
            l3: b[9] & 0x40 != 0,
            r3: b[9] & 0x80 != 0,
            ps: b[10] & 0x01 != 0,
            touchpad: b[10] & 0x02 != 0,
            mute: b[10] & 0x04 != 0,
        }
    }

    pub fn set_buttons(&mut self, bt: &Buttons) {
        self.raw[8] = (bt.dpad & 0x0F)
            | (bt.square as u8) << 4
            | (bt.cross as u8) << 5
            | (bt.circle as u8) << 6
            | (bt.triangle as u8) << 7;
        self.raw[9] = (bt.l1 as u8)
            | (bt.r1 as u8) << 1
            | (bt.l2 as u8) << 2
            | (bt.r2 as u8) << 3
            | (bt.share as u8) << 4
            | (bt.options as u8) << 5
            | (bt.l3 as u8) << 6
            | (bt.r3 as u8) << 7;
        self.raw[10] = (bt.ps as u8) | (bt.touchpad as u8) << 1 | (bt.mute as u8) << 2;
    }

    /// Battery level in the low nibble of byte 53 (0..=10 observed; the real pad also reports
    /// charging state in the high nibble). Exposed read-only for status display.
    pub fn battery_raw(&self) -> u8 {
        self.raw[53]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_report_is_well_formed() {
        let r = InputReport::neutral();
        assert_eq!(r.sticks(), (0x80, 0x80, 0x80, 0x80));
        assert_eq!(r.triggers(), (0, 0));
        assert_eq!(r.buttons().dpad, 8);
        assert_eq!(r.raw[32], 14);
        assert_eq!(r.raw[53], 42);
        assert_eq!(r.raw[54], 8);
    }

    #[test]
    fn parse_rejects_bad_length_and_id() {
        assert!(InputReport::parse(&[0u8; 63]).is_err());
        assert!(InputReport::parse(&[0u8; 65]).is_err());
        assert!(InputReport::parse(&[0x02; 64]).is_err());
        assert!(InputReport::parse(InputReport::neutral().as_bytes()).is_ok());
    }

    #[test]
    fn button_round_trip() {
        let mut r = InputReport::neutral();
        let mut bt = Buttons::default();
        bt.dpad = 1;
        bt.cross = true;
        bt.triangle = true;
        bt.r2 = true;
        bt.options = true;
        bt.l3 = true;
        bt.ps = true;
        bt.mute = true;
        r.set_buttons(&bt);
        assert_eq!(r.buttons(), bt);
        // Bit assignments (as in Linux hid-playstation): byte 8 dpad|face, 9 shoulders/etc, 10 PS/touch/mute.
        assert_eq!(r.raw[8], 0x01 | 0x20 | 0x80);
        assert_eq!(r.raw[9], 0x08 | 0x20 | 0x40);
        assert_eq!(r.raw[10], 0x01 | 0x04);
    }

    #[test]
    fn sticks_and_triggers_round_trip() {
        let mut r = InputReport::neutral();
        r.set_sticks(0, 255, 12, 200);
        r.set_triggers(10, 250);
        let p = InputReport::parse(r.as_bytes()).unwrap();
        assert_eq!(p.sticks(), (0, 255, 12, 200));
        assert_eq!(p.triggers(), (10, 250));
    }

    #[test]
    fn untouched_fields_are_preserved_on_parse() {
        let mut raw = InputReport::neutral().raw;
        raw[16..28].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]); // gyro/accel
        raw[41] = 77; // touchpad packet counter
        let r = InputReport::parse(&raw).unwrap();
        assert_eq!(r.as_bytes(), &raw);
    }
}
