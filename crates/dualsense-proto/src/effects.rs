//! Effects Periph itself sets on the controller: adaptive trigger effects, lightbar colour,
//! player LEDs and the mic LED, written into output report 0x02 with only their sections
//! flagged valid.
//!
//! Every encoding here was checked against output reports captured from DSX (Frida HID hook,
//! `captures/dsx-effects.jsonl`, git-ignored): the tests below compare each preset with the
//! exact 11-byte block DSX sent. Parameter ranges follow the DSX UI (positions 0–9, strengths and
//! amplitudes 1–8, frequencies 1–10), so nothing outside what DSX can send is produced.
//!
//! Trigger block layout (11 bytes, right trigger at report offset 11, left at 22):
//! - byte 0: mode;
//! - zone modes (0x21 feedback, 0x26 vibration): bytes 1–2 = 10-zone active mask (u16 LE),
//!   bytes 3–6 = 3-bit strength per zone (u32 LE, value = strength − 1), byte 9 = frequency;
//! - two-position modes (0x22 bow, 0x23 galloping, 0x25 weapon, 0x27 machine): bytes 1–2 =
//!   mask with the start and end bits, then mode-specific packed fields;
//! - 0x02 (GameCube preset) and the raw "custom" modes use plain bytes.

use crate::output::{OutputReport, LEFT_TRIGGER_OFFSET, RIGHT_TRIGGER_OFFSET, TRIGGER_BLOCK_LEN};
use crate::OUTPUT_REPORT_ID;

pub type TriggerBlock = [u8; TRIGGER_BLOCK_LEN];

/// Raw mode bytes DSX's "custom" trigger page sent (rigid, rigid B, rigid A, A on L2, rigid AB).
pub const CUSTOM_MODES: &[u8] = &[0x01, 0x05, 0x21, 0x22, 0x25];

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase"))]
pub enum TriggerEffect {
    /// No resistance (mode 0x05).
    Off,
    /// Constant resistance from `start` (0–9) to the end of travel.
    Feedback { start: u8, strength: u8 },
    /// Resistance between `start` (2–7) and `end` (start+1–8) that "breaks" like a trigger.
    Weapon { start: u8, end: u8, strength: u8 },
    /// Vibration from `start` (0–9) to the end of travel.
    Vibration { start: u8, amplitude: u8, frequency: u8 },
    /// Resistance rising or falling linearly between two positions.
    Slope { start: u8, end: u8, start_strength: u8, end_strength: u8 },
    /// Free per-zone resistance: 0 = zone off, 1–8 = strength.
    MultiFeedback { strengths: [u8; 10] },
    /// Free per-zone vibration: 0 = zone off, 1–8 = amplitude.
    MultiVibration { amplitudes: [u8; 10], frequency: u8 },
    /// Tension that releases with a snap at `end`.
    Bow { start: u8, end: u8, strength: u8, snap: u8 },
    /// Galloping rhythm; the two foot timings are 3-bit values.
    Galloping { start: u8, end: u8, first_foot: u8, second_foot: u8, frequency: u8 },
    /// Two alternating vibration amplitudes.
    Machine { start: u8, end: u8, amplitude_a: u8, amplitude_b: u8, frequency: u8, period: u8 },
    /// DSX "GameCube" preset (mode 0x02, fixed bytes).
    GameCube,
    /// DSX custom page: raw mode byte (see [`CUSTOM_MODES`]) and 7 force bytes.
    Custom { mode: u8, forces: [u8; 7] },
}

fn check(ok: bool, what: &'static str) -> Result<(), &'static str> {
    if ok {
        Ok(())
    } else {
        Err(what)
    }
}

fn range(v: u8, lo: u8, hi: u8, what: &'static str) -> Result<(), &'static str> {
    check((lo..=hi).contains(&v), what)
}

/// Encodes a zone mode (0x21 / 0x26) from per-zone values (0 = zone off, 1–8).
fn zones(mode: u8, values: &[u8; 10], frequency: u8) -> TriggerBlock {
    let mut mask = 0u16;
    let mut forces = 0u32;
    for (i, &v) in values.iter().enumerate() {
        if v > 0 {
            mask |= 1 << i;
            forces |= u32::from(v - 1) << (3 * i);
        }
    }
    let mut b = [0u8; TRIGGER_BLOCK_LEN];
    b[0] = mode;
    b[1..3].copy_from_slice(&mask.to_le_bytes());
    b[3..7].copy_from_slice(&forces.to_le_bytes());
    b[9] = frequency;
    b
}

/// Mode byte + mask with the start and end position bits.
fn two_positions(mode: u8, start: u8, end: u8) -> TriggerBlock {
    let mask: u16 = (1 << start) | (1 << end);
    let mut b = [0u8; TRIGGER_BLOCK_LEN];
    b[0] = mode;
    b[1..3].copy_from_slice(&mask.to_le_bytes());
    b
}

/// Per-zone strengths of a slope: linear from `start` to `end`, rounded half-to-even (matches
/// DSX: 8→1 over zones 1–9 gives 8,7,6,5,4,4,3,2,1), and `end_strength` after `end`.
pub fn slope_strengths(start: u8, end: u8, start_strength: u8, end_strength: u8) -> [u8; 10] {
    let mut s = [0u8; 10];
    let step = (f32::from(end_strength) - f32::from(start_strength)) / f32::from(end - start);
    for i in start..10 {
        s[i as usize] = if i <= end {
            (f32::from(start_strength) + step * f32::from(i - start)).round_ties_even() as u8
        } else {
            end_strength
        };
    }
    s
}

impl TriggerEffect {
    /// Validates the parameters against the DSX ranges and builds the 11-byte block.
    pub fn encode(&self) -> Result<TriggerBlock, &'static str> {
        use TriggerEffect::*;
        Ok(match *self {
            Off => {
                let mut b = [0u8; TRIGGER_BLOCK_LEN];
                b[0] = 0x05;
                b
            }
            Feedback { start, strength } => {
                range(start, 0, 9, "feedback start must be 0-9")?;
                range(strength, 1, 8, "feedback strength must be 1-8")?;
                let mut v = [0u8; 10];
                v[start as usize..].fill(strength);
                zones(0x21, &v, 0)
            }
            Weapon { start, end, strength } => {
                range(start, 2, 7, "weapon start must be 2-7")?;
                check(end > start && end <= 8, "weapon end must be after start, up to 8")?;
                range(strength, 1, 8, "weapon strength must be 1-8")?;
                let mut b = two_positions(0x25, start, end);
                b[3] = strength - 1;
                b
            }
            Vibration { start, amplitude, frequency } => {
                range(start, 0, 9, "vibration start must be 0-9")?;
                range(amplitude, 1, 8, "vibration amplitude must be 1-8")?;
                range(frequency, 1, 10, "vibration frequency must be 1-10")?;
                let mut v = [0u8; 10];
                v[start as usize..].fill(amplitude);
                zones(0x26, &v, frequency)
            }
            Slope { start, end, start_strength, end_strength } => {
                check(start < end && end <= 9, "slope needs start < end <= 9")?;
                range(start_strength, 1, 8, "slope strengths must be 1-8")?;
                range(end_strength, 1, 8, "slope strengths must be 1-8")?;
                zones(0x21, &slope_strengths(start, end, start_strength, end_strength), 0)
            }
            MultiFeedback { strengths } => {
                check(strengths.iter().all(|&s| s <= 8), "zone strengths must be 0-8")?;
                check(strengths.iter().any(|&s| s > 0), "at least one zone must be on")?;
                zones(0x21, &strengths, 0)
            }
            MultiVibration { amplitudes, frequency } => {
                check(amplitudes.iter().all(|&s| s <= 8), "zone amplitudes must be 0-8")?;
                check(amplitudes.iter().any(|&s| s > 0), "at least one zone must be on")?;
                range(frequency, 1, 10, "vibration frequency must be 1-10")?;
                zones(0x26, &amplitudes, frequency)
            }
            Bow { start, end, strength, snap } => {
                check(start < end && end <= 8, "bow needs start < end <= 8")?;
                range(strength, 1, 8, "bow strength must be 1-8")?;
                range(snap, 1, 8, "bow snap must be 1-8")?;
                let mut b = two_positions(0x22, start, end);
                b[3] = (strength - 1) | ((snap - 1) << 3);
                b
            }
            Galloping { start, end, first_foot, second_foot, frequency } => {
                check(start < end && end <= 9, "galloping needs start < end <= 9")?;
                check(first_foot < second_foot && second_foot <= 7, "galloping needs first foot < second foot <= 7")?;
                range(frequency, 1, 10, "galloping frequency must be 1-10")?;
                let mut b = two_positions(0x23, start, end);
                b[3] = (first_foot << 3) | second_foot;
                b[4] = frequency;
                b
            }
            Machine { start, end, amplitude_a, amplitude_b, frequency, period } => {
                check(start < end && end <= 9, "machine needs start < end <= 9")?;
                check(amplitude_a <= 7 && amplitude_b <= 7, "machine amplitudes must be 0-7")?;
                range(frequency, 1, 10, "machine frequency must be 1-10")?;
                range(period, 0, 10, "machine period must be 0-10")?;
                let mut b = two_positions(0x27, start, end);
                b[3] = (amplitude_b << 3) | amplitude_a;
                b[4] = frequency;
                b[5] = period;
                b
            }
            GameCube => {
                let mut b = [0u8; TRIGGER_BLOCK_LEN];
                b[..4].copy_from_slice(&[0x02, 0x90, 0xA0, 0xFF]);
                b
            }
            Custom { mode, forces } => {
                check(CUSTOM_MODES.contains(&mode), "custom mode not supported")?;
                let mut b = [0u8; TRIGGER_BLOCK_LEN];
                b[0] = mode;
                b[1..8].copy_from_slice(&forces);
                b
            }
        })
    }
}

/// Player LED brightness byte (report offset 43): 0 high, 1 medium, 2 low.
pub const PLAYER_BRIGHTNESS_MAX: u8 = 2;

/// Sections Periph writes into an output report. `None` = section not touched (its valid flag
/// stays clear, so the controller keeps whatever it had).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sections {
    pub lightbar: Option<[u8; 3]>,
    /// (5-bit LED mask, brightness 0–2)
    pub player_leds: Option<(u8, u8)>,
    /// 0 off, 1 on
    pub mic_led: Option<u8>,
    pub right_trigger: Option<TriggerBlock>,
    pub left_trigger: Option<TriggerBlock>,
    /// Lightbar setup "light out": releases the lightbar from the controller's boot animation.
    /// Send it once per connection, on its own (as Linux hid-playstation does at init); repeated
    /// with every colour it restarts a fade-out and the colour never settles.
    pub release_lightbar: bool,
}

impl Sections {
    pub fn is_empty(&self) -> bool {
        *self == Sections::default()
    }

    /// Writes every present section and sets its valid flags; other bytes are left alone.
    /// Flags as in Linux hid-playstation and DSX: byte 1 bit 2/3 = right/left trigger; byte 2
    /// bit 0 mic LED, bit 2 lightbar, bit 4 player LEDs; byte 39 bit 0 LED brightness control,
    /// bit 1 lightbar setup control with byte 42 = 0x02 (only with `release_lightbar`).
    pub fn apply(&self, report: &mut OutputReport) {
        let r = &mut report.raw;
        if let Some(block) = self.right_trigger {
            r[1] |= 0x04;
            r[RIGHT_TRIGGER_OFFSET..RIGHT_TRIGGER_OFFSET + TRIGGER_BLOCK_LEN].copy_from_slice(&block);
        }
        if let Some(block) = self.left_trigger {
            r[1] |= 0x08;
            r[LEFT_TRIGGER_OFFSET..LEFT_TRIGGER_OFFSET + TRIGGER_BLOCK_LEN].copy_from_slice(&block);
        }
        if let Some(on) = self.mic_led {
            r[2] |= 0x01;
            r[9] = on.min(1);
        }
        if let Some([red, green, blue]) = self.lightbar {
            r[2] |= 0x04;
            r[45] = red;
            r[46] = green;
            r[47] = blue;
        }
        if self.release_lightbar {
            r[39] |= 0x02;
            r[42] = 0x02;
        }
        if let Some((mask, brightness)) = self.player_leds {
            r[2] |= 0x10;
            r[39] |= 0x01;
            r[43] = brightness.min(PLAYER_BRIGHTNESS_MAX);
            r[44] = mask & 0x1F;
        }
    }

    /// A fresh report carrying only these sections.
    pub fn report(&self) -> OutputReport {
        let mut raw = [0u8; crate::OUTPUT_REPORT_LEN];
        raw[0] = OUTPUT_REPORT_ID;
        let mut report = OutputReport { raw };
        self.apply(&mut report);
        report
    }
}

/// Which sections a (game) output report sets, from its valid flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SectionFlags {
    pub lightbar: bool,
    pub player_leds: bool,
    pub mic_led: bool,
    pub right_trigger: bool,
    pub left_trigger: bool,
}

impl SectionFlags {
    pub fn of(report: &OutputReport) -> Self {
        let r = &report.raw;
        Self {
            right_trigger: r[1] & 0x04 != 0,
            left_trigger: r[1] & 0x08 != 0,
            mic_led: r[2] & 0x01 != 0,
            lightbar: r[2] & 0x04 != 0,
            player_leds: r[2] & 0x10 != 0,
        }
    }

    pub fn union(self, o: Self) -> Self {
        Self {
            lightbar: self.lightbar || o.lightbar,
            player_leds: self.player_leds || o.player_leds,
            mic_led: self.mic_led || o.mic_led,
            right_trigger: self.right_trigger || o.right_trigger,
            left_trigger: self.left_trigger || o.left_trigger,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TriggerEffect::*;

    /// Parses "21 fe 03 ..." into an 11-byte block (missing bytes are zero).
    fn block(hex: &str) -> TriggerBlock {
        let mut b = [0u8; TRIGGER_BLOCK_LEN];
        for (i, x) in hex.split_whitespace().enumerate() {
            b[i] = u8::from_str_radix(x, 16).unwrap();
        }
        b
    }

    /// Each case: the DSX setting, the effect Periph builds for it, and the block DSX sent.
    #[test]
    fn presets_match_the_dsx_captures() {
        let cases: &[(&str, TriggerEffect, &str)] = &[
            ("off", Off, "05"),
            ("continuous resistance", Feedback { start: 1, strength: 8 }, "21 fe 03 f8 ff ff 3f"),
            ("weapon", Weapon { start: 2, end: 6, strength: 8 }, "25 44 00 07"),
            ("vibration", Vibration { start: 1, amplitude: 8, frequency: 10 }, "26 fe 03 f8 ff ff 3f 00 00 0a"),
            ("slope", Slope { start: 1, end: 9, start_strength: 8, end_strength: 1 }, "21 fe 03 b8 cb 4d 01"),
            ("multi-position resistance", MultiFeedback { strengths: [8, 8, 8, 0, 0, 0, 8, 8, 0, 0] }, "21 c7 00 ff 01 fc 00"),
            (
                "multi-position vibration",
                MultiVibration { amplitudes: [8, 8, 8, 8, 8, 0, 0, 0, 8, 8], frequency: 10 },
                "26 1f 03 ff 7f 00 3f 00 00 0a",
            ),
            ("gamecube", GameCube, "02 90 a0 ff"),
            ("bow", Bow { start: 1, end: 8, strength: 5, snap: 8 }, "22 02 01 3c"),
            ("galloping", Galloping { start: 1, end: 9, first_foot: 3, second_foot: 6, frequency: 4 }, "23 02 02 1e 04"),
            (
                "machine",
                Machine { start: 1, end: 9, amplitude_a: 2, amplitude_b: 7, frequency: 10, period: 5 },
                "27 02 02 3a 0a 05",
            ),
            ("custom rigid A", Custom { mode: 0x21, forces: [0xff; 7] }, "21 ff ff ff ff ff ff ff"),
            ("custom rigid A, force 1 = 0", Custom { mode: 0x21, forces: [0, 255, 255, 255, 255, 255, 255] }, "21 00 ff ff ff ff ff ff"),
            ("custom rigid A, force 7 = 0", Custom { mode: 0x21, forces: [255, 255, 255, 255, 255, 255, 0] }, "21 ff ff ff ff ff ff 00"),
            ("custom rigid B", Custom { mode: 0x05, forces: [0xff; 7] }, "05 ff ff ff ff ff ff ff"),
            ("custom rigid AB", Custom { mode: 0x25, forces: [0xff; 7] }, "25 ff ff ff ff ff ff ff"),
            ("custom on L2", Custom { mode: 0x22, forces: [0xff; 7] }, "22 ff ff ff ff ff ff ff"),
        ];
        for (name, effect, expected) in cases {
            assert_eq!(effect.encode().unwrap(), block(expected), "{name}");
        }
    }

    #[test]
    fn slope_rounds_like_dsx() {
        assert_eq!(slope_strengths(1, 9, 8, 1), [0, 8, 7, 6, 5, 4, 4, 3, 2, 1]);
        // after `end` the end strength continues
        assert_eq!(slope_strengths(0, 4, 1, 5), [1, 2, 3, 4, 5, 5, 5, 5, 5, 5]);
    }

    #[test]
    fn out_of_range_parameters_are_rejected() {
        let bad = [
            Feedback { start: 10, strength: 8 },
            Feedback { start: 0, strength: 0 },
            Feedback { start: 0, strength: 9 },
            Weapon { start: 1, end: 6, strength: 8 },
            Weapon { start: 5, end: 5, strength: 8 },
            Weapon { start: 2, end: 9, strength: 8 },
            Vibration { start: 0, amplitude: 8, frequency: 0 },
            Vibration { start: 0, amplitude: 8, frequency: 11 },
            Slope { start: 5, end: 5, start_strength: 1, end_strength: 8 },
            MultiFeedback { strengths: [0; 10] },
            MultiFeedback { strengths: [9, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
            Bow { start: 0, end: 9, strength: 1, snap: 1 },
            Galloping { start: 0, end: 9, first_foot: 6, second_foot: 3, frequency: 4 },
            Machine { start: 0, end: 9, amplitude_a: 8, amplitude_b: 0, frequency: 1, period: 0 },
            Machine { start: 0, end: 9, amplitude_a: 1, amplitude_b: 0, frequency: 1, period: 11 },
            Custom { mode: 0x27, forces: [0; 7] },
        ];
        for effect in bad {
            assert!(effect.encode().is_err(), "{effect:?}");
        }
    }

    #[test]
    fn sections_set_only_their_flags() {
        let s = Sections {
            lightbar: Some([0x9d, 0, 0]),
            right_trigger: Some(Weapon { start: 2, end: 6, strength: 8 }.encode().unwrap()),
            ..Default::default()
        };
        let r = s.report();
        assert_eq!(r.raw[0], 0x02);
        assert_eq!(r.raw[1], 0x04, "right trigger only, no rumble bits");
        assert_eq!(r.raw[2], 0x04, "lightbar only");
        assert_eq!((r.raw[39], r.raw[42]), (0, 0), "no lightbar setup with a colour");
        assert_eq!(r.lightbar_rgb(), (0x9d, 0, 0));
        assert_eq!(&r.right_trigger()[..4], &[0x25, 0x44, 0x00, 0x07]);
        assert_eq!(r.left_trigger()[0], 0, "left block untouched");
        let flags = SectionFlags::of(&r);
        assert!(flags.lightbar && flags.right_trigger && !flags.left_trigger && !flags.player_leds && !flags.mic_led);
    }

    #[test]
    fn overlay_keeps_the_games_other_sections() {
        // A game report setting rumble + lightbar; Periph overrides the right trigger only.
        let mut raw = [0u8; 48];
        raw[0] = 0x02;
        raw[1] = 0x01;
        raw[2] = 0x04;
        raw[3] = 200;
        raw[45] = 7;
        let mut game = OutputReport::parse(&raw).unwrap();
        Sections { right_trigger: Some(Off.encode().unwrap()), ..Default::default() }.apply(&mut game);
        assert_eq!(game.raw[1], 0x05);
        assert_eq!(game.raw[3], 200);
        assert_eq!(game.lightbar_rgb(), (7, 0, 0));
        assert_eq!(game.right_trigger()[0], 0x05);
    }

    #[test]
    fn lightbar_release_is_its_own_flag() {
        let r = Sections { release_lightbar: true, ..Default::default() }.report();
        assert_eq!((r.raw[2], r.raw[39], r.raw[42]), (0, 0x02, 0x02));
    }

    #[test]
    fn player_leds_and_mic() {
        let r = Sections { player_leds: Some((0xFF, 9)), mic_led: Some(5), ..Default::default() }.report();
        assert_eq!(r.raw[2], 0x11);
        assert_eq!(r.raw[39], 0x01);
        assert_eq!(r.raw[43], 2, "brightness clamped to low");
        assert_eq!(r.raw[44], 0x1F, "only 5 LEDs");
        assert_eq!(r.raw[9], 1, "mic on/off only");
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::TriggerEffect;

    #[test]
    fn json_shape_for_the_ui() {
        let e: TriggerEffect = serde_json::from_str(r#"{"kind":"weapon","start":2,"end":6,"strength":8}"#).unwrap();
        assert_eq!(e, TriggerEffect::Weapon { start: 2, end: 6, strength: 8 });
        let e: TriggerEffect = serde_json::from_str(r#"{"kind":"machine","start":1,"end":9,"amplitudeA":2,"amplitudeB":7,"frequency":10,"period":5}"#).unwrap();
        assert!(matches!(e, TriggerEffect::Machine { amplitude_a: 2, amplitude_b: 7, .. }));
        assert_eq!(serde_json::to_string(&TriggerEffect::Off).unwrap(), r#"{"kind":"off"}"#);
        assert_eq!(serde_json::to_string(&TriggerEffect::GameCube).unwrap(), r#"{"kind":"gameCube"}"#);
    }
}
