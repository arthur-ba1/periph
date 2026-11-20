//! USB output report 0x02 (48 bytes): rumble, lightbar, player/mic LEDs and adaptive triggers.
//! This is what a game writes to the controller; the virtual device forwards it to the physical
//! one. Offsets and valid flags follow `docs/dualsense-protocol.md` (same layout as Linux
//! `hid-playstation`).
//!
//! Every section is applied by the controller only when its valid-flag bit is set, so a report
//! with only the rumble bit set changes the motors and nothing else.

use crate::{OUTPUT_REPORT_ID, OUTPUT_REPORT_LEN};

/// 11-byte adaptive trigger block (mode + 10 parameter bytes), right at offset 11, left at 22.
pub const TRIGGER_BLOCK_LEN: usize = 11;
pub const RIGHT_TRIGGER_OFFSET: usize = 11;
pub const LEFT_TRIGGER_OFFSET: usize = 22;

/// valid_flag0 (byte 1) bits that make the controller apply the motor bytes:
/// bit 0 compatible vibration, bit 1 haptics select.
pub const VALID0_RUMBLE: u8 = 0x03;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputReport {
    pub raw: [u8; OUTPUT_REPORT_LEN],
}

impl OutputReport {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != OUTPUT_REPORT_LEN {
            return Err("output report must be 48 bytes");
        }
        if bytes[0] != OUTPUT_REPORT_ID {
            return Err("output report id must be 0x02");
        }
        let mut raw = [0u8; OUTPUT_REPORT_LEN];
        raw.copy_from_slice(bytes);
        Ok(Self { raw })
    }

    pub fn as_bytes(&self) -> &[u8; OUTPUT_REPORT_LEN] {
        &self.raw
    }

    /// (right motor, left motor).
    pub fn rumble(&self) -> (u8, u8) {
        (self.raw[3], self.raw[4])
    }

    pub fn mic_led(&self) -> u8 {
        self.raw[9]
    }

    pub fn lightbar_rgb(&self) -> (u8, u8, u8) {
        (self.raw[45], self.raw[46], self.raw[47])
    }

    pub fn player_leds(&self) -> u8 {
        self.raw[44]
    }

    pub fn right_trigger(&self) -> &[u8] {
        &self.raw[RIGHT_TRIGGER_OFFSET..RIGHT_TRIGGER_OFFSET + TRIGGER_BLOCK_LEN]
    }

    pub fn left_trigger(&self) -> &[u8] {
        &self.raw[LEFT_TRIGGER_OFFSET..LEFT_TRIGGER_OFFSET + TRIGGER_BLOCK_LEN]
    }

    /// How this report changes the motors: `Some(true)` starts or keeps vibration,
    /// `Some(false)` stops it, `None` leaves the motors as they were (rumble bits not set).
    pub fn rumble_update(&self) -> Option<bool> {
        (self.raw[1] & VALID0_RUMBLE != 0).then(|| self.raw[3] != 0 || self.raw[4] != 0)
    }

    /// A report that stops both motors and touches nothing else: only the compatible-vibration
    /// bit is set, with both motors at 0; lightbar, LEDs and triggers keep their current state.
    pub fn rumble_stop() -> Self {
        let mut raw = [0u8; OUTPUT_REPORT_LEN];
        raw[0] = OUTPUT_REPORT_ID;
        raw[1] = 0x01;
        Self { raw }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_bad_input() {
        assert!(OutputReport::parse(&[0u8; 47]).is_err());
        assert!(OutputReport::parse(&[0u8; 49]).is_err());
        assert!(OutputReport::parse(&[0x01; 48]).is_err());
    }

    #[test]
    fn sections_read_at_documented_offsets() {
        let mut raw = [0u8; 48];
        raw[0] = 0x02;
        raw[3] = 0xAA; // right motor
        raw[4] = 0x55; // left motor
        raw[9] = 1; // mic led on
        raw[11] = 0x21; // right trigger mode
        raw[22] = 0x05; // left trigger mode (off family)
        raw[44] = 0x1F;
        raw[45] = 10;
        raw[46] = 20;
        raw[47] = 30;
        let r = OutputReport::parse(&raw).unwrap();
        assert_eq!(r.rumble(), (0xAA, 0x55));
        assert_eq!(r.mic_led(), 1);
        assert_eq!(r.lightbar_rgb(), (10, 20, 30));
        assert_eq!(r.player_leds(), 0x1F);
        assert_eq!(r.right_trigger()[0], 0x21);
        assert_eq!(r.left_trigger()[0], 0x05);
        assert_eq!(r.rumble_update(), None); // rumble bits not set: motors untouched
    }

    #[test]
    fn rumble_update_follows_the_valid_flags() {
        let mut raw = [0u8; 48];
        raw[0] = 0x02;
        raw[1] = 0x01;
        raw[3] = 200;
        assert_eq!(OutputReport::parse(&raw).unwrap().rumble_update(), Some(true));
        raw[3] = 0;
        assert_eq!(OutputReport::parse(&raw).unwrap().rumble_update(), Some(false));
        raw[1] = 0x02; // haptics select alone also carries the motor bytes
        raw[4] = 7;
        assert_eq!(OutputReport::parse(&raw).unwrap().rumble_update(), Some(true));
    }

    #[test]
    fn rumble_stop_touches_nothing_else() {
        let stop = OutputReport::rumble_stop();
        assert_eq!(stop.rumble_update(), Some(false));
        assert_eq!(stop.raw[0], 0x02);
        assert_eq!(stop.raw[1], 0x01);
        // No other section is flagged valid: valid_flag1 (byte 2) and valid_flag2 (byte 39).
        assert_eq!(stop.raw[2], 0);
        assert_eq!(stop.raw[39], 0);
        assert!(stop.raw[2..].iter().all(|&b| b == 0));
    }
}
