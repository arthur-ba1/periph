//! Periph's own controller effects: lightbar (static or animated), player LEDs, mic LED and
//! adaptive trigger effects, and how they coexist with a game's feedback while emulating.
//!
//! Policy (chosen by the user per category in the Emulation page, `Feedback`): for the lights
//! (lightbar, player and mic LEDs) and the triggers, either **the game** or **Periph** has the
//! last word while emulating. Game: what the game sets reaches the pad, and Periph's value only
//! fills the sections the game never touched. Periph: the game's values for that category are
//! dropped from every forwarded report and Periph's are written instead. The game's vibration
//! is either forwarded or dropped. The DualSense keeps no state of its own, so Periph re-sends
//! its sections periodically (and every frame while a lightbar animation runs).

use anyhow::{bail, Result};
use dualsense_proto::effects::{SectionFlags, Sections, TriggerEffect, PLAYER_BRIGHTNESS_MAX};
use dualsense_proto::output::OutputReport;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Frame interval of lightbar animations.
pub const FRAME: Duration = Duration::from_millis(33);
/// Static sections are re-sent this often (another app may have changed them).
const KEEPALIVE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LightMode {
    /// Periph does not touch the lightbar (never set; the game or the pad decide).
    #[default]
    None,
    /// The lightbar switched off: kept dark (black).
    Off,
    Static,
    Breathing,
    Strobe,
    Rainbow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Lightbar {
    pub mode: LightMode,
    pub color: [u8; 3],
    /// 0–100 %, applied in software (the controller has no brightness setting for it).
    pub brightness: u8,
    /// Animation speed 1–10.
    pub speed: u8,
}

impl Default for Lightbar {
    fn default() -> Self {
        Self { mode: LightMode::None, color: [0, 80, 255], brightness: 100, speed: 5 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlayerLeds {
    pub enabled: bool,
    /// 5 LEDs, bit 0 = leftmost.
    pub mask: u8,
    /// 0 high, 1 medium, 2 low.
    pub brightness: u8,
}

impl Default for PlayerLeds {
    fn default() -> Self {
        Self { enabled: false, mask: 0x04, brightness: 0 }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MicMode {
    /// Never set: Periph leaves the LED alone (the UI shows it as off, the pad's own state).
    #[default]
    Auto,
    Off,
    On,
    /// On and off for `interval_ms` each (timed by Periph: the LED's own pulse has no speed).
    Blink,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MicLed {
    pub mode: MicMode,
    pub interval_ms: u16,
}

impl Default for MicLed {
    fn default() -> Self {
        Self { mode: MicMode::Auto, interval_ms: 500 }
    }
}

pub const MIC_INTERVAL_MS: std::ops::RangeInclusive<u16> = 100..=2000;

impl MicLed {
    pub fn validate(&self) -> Result<()> {
        if !MIC_INTERVAL_MS.contains(&self.interval_ms) {
            bail!("mic LED blink interval must be 100-2000 ms");
        }
        Ok(())
    }

    /// LED state (0 off, 1 on) `t` seconds after the engine started; `None` in `Auto`.
    pub fn value_at(&self, t: f32) -> Option<u8> {
        match self.mode {
            MicMode::Auto => None,
            MicMode::Off => Some(0),
            MicMode::On => Some(1),
            MicMode::Blink => Some(u8::from((t * 1000.0 / f32::from(self.interval_ms.max(1))) as u64 % 2 == 0)),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Trigger {
    /// `None` = Periph does not touch this trigger.
    pub effect: Option<TriggerEffect>,
}

/// Who has the last word on one category while emulating.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    #[default]
    Game,
    Periph,
}

/// What of the game's feedback reaches the physical pad while emulating; part of the config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Feedback {
    /// Lightbar, player LEDs and mic LED.
    pub lights: Source,
    /// The game's vibration reaches the pad (Periph has none of its own).
    pub haptics: bool,
    /// Both adaptive triggers.
    pub triggers: Source,
}

impl Default for Feedback {
    fn default() -> Self {
        Self { lights: Source::Game, haptics: true, triggers: Source::Game }
    }
}

impl Feedback {
    /// Feedback that forwards nothing (the old "forward feedback: off").
    pub const NONE: Self = Self { lights: Source::Periph, haptics: false, triggers: Source::Periph };

    /// Drops from a game report what the user keeps for Periph (or turned off).
    pub fn filter(&self, report: &mut OutputReport) {
        if self.lights == Source::Periph {
            report.drop_lights();
        }
        if !self.haptics {
            report.drop_rumble();
        }
        if self.triggers == Source::Periph {
            report.drop_triggers();
        }
    }

    fn periph(&self, section: Section) -> bool {
        match section {
            Section::Lightbar | Section::PlayerLeds | Section::MicLed => self.lights == Source::Periph,
            Section::Left | Section::Right => self.triggers == Source::Periph,
        }
    }
}

/// Every effect setting; part of the driver config and saved to disk.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Effects {
    pub lightbar: Lightbar,
    pub player_leds: PlayerLeds,
    pub mic_led: MicLed,
    pub left_trigger: Trigger,
    pub right_trigger: Trigger,
}

impl Lightbar {
    pub fn validate(&self) -> Result<()> {
        if self.brightness > 100 || !(1..=10).contains(&self.speed) {
            bail!("lightbar brightness must be 0-100 and speed 1-10");
        }
        Ok(())
    }

    pub fn animated(&self) -> bool {
        matches!(self.mode, LightMode::Breathing | LightMode::Strobe | LightMode::Rainbow)
    }

    /// Colour to show `t` seconds after the animation started.
    pub fn color_at(&self, t: f32) -> Option<[u8; 3]> {
        let period = 3.25 - 0.25 * f32::from(self.speed); // speed 1 → 3 s, 10 → 0.75 s
        let phase = (t / period).fract();
        let (base, level) = match self.mode {
            LightMode::None => return None,
            LightMode::Off => ([0, 0, 0], 0.0),
            LightMode::Static => (self.color, 1.0),
            LightMode::Breathing => (self.color, 0.5 - 0.5 * (std::f32::consts::TAU * phase).cos()),
            LightMode::Strobe => (self.color, if phase < 0.5 { 1.0 } else { 0.0 }),
            LightMode::Rainbow => (hue(phase), 1.0),
        };
        let k = level * f32::from(self.brightness) / 100.0;
        Some(base.map(|c| (f32::from(c) * k).round() as u8))
    }
}

/// Fully saturated colour for a hue in [0, 1).
fn hue(h: f32) -> [u8; 3] {
    let x = h * 6.0;
    let f = x.fract();
    let (r, g, b) = match x as u32 % 6 {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    [r, g, b].map(|v: f32| (v * 255.0).round() as u8)
}

impl PlayerLeds {
    pub fn validate(&self) -> Result<()> {
        if self.mask > 0x1F || self.brightness > PLAYER_BRIGHTNESS_MAX {
            bail!("player LEDs: mask must be 5 bits and brightness 0-2");
        }
        Ok(())
    }
}

impl Trigger {
    pub fn validate(&self) -> Result<()> {
        if let Some(effect) = &self.effect {
            effect.encode().map_err(anyhow::Error::msg)?;
        }
        Ok(())
    }
}

/// Runtime state: what the game has taken over, pending trigger releases, send timing.
#[derive(Debug)]
pub struct Engine {
    started: Instant,
    last_sent: Option<Instant>,
    dirty: bool,
    /// Sections the game has set since emulation started (in categories left to the game).
    pub game_owns: SectionFlags,
    /// Triggers to reset to "off" once (Periph stopped controlling them, or the game left).
    release: SectionFlags,
    /// The lightbar was released from the boot animation (once per connection).
    lightbar_released: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            last_sent: None,
            dirty: true,
            game_owns: SectionFlags::default(),
            release: SectionFlags::default(),
            lightbar_released: false,
        }
    }
}

impl Engine {
    /// Something changed: send on the next poll.
    pub fn touch(&mut self) {
        self.dirty = true;
    }

    /// A trigger stopped being controlled by Periph: reset it to off once.
    pub fn release_trigger(&mut self, left: bool) {
        if left {
            self.release.left_trigger = true;
        } else {
            self.release.right_trigger = true;
        }
        self.dirty = true;
    }

    /// Emulation stopped: triggers the game had set and Periph does not control go back to off,
    /// and Periph takes its sections back.
    pub fn game_left(&mut self, fx: &Effects) {
        if self.game_owns.left_trigger && fx.left_trigger.effect.is_none() {
            self.release.left_trigger = true;
        }
        if self.game_owns.right_trigger && fx.right_trigger.effect.is_none() {
            self.release.right_trigger = true;
        }
        self.game_owns = SectionFlags::default();
        self.dirty = true;
    }

    fn t(&self, now: Instant) -> f32 {
        now.duration_since(self.started).as_secs_f32()
    }

    /// A category was handed to Periph: it takes those sections back from the game, and
    /// triggers the game had set that Periph does not control go back to off.
    pub fn take_back(&mut self, fx: &Effects, fb: &Feedback) {
        let owns = &mut self.game_owns;
        if fb.triggers == Source::Periph {
            if owns.left_trigger && fx.left_trigger.effect.is_none() {
                self.release.left_trigger = true;
            }
            if owns.right_trigger && fx.right_trigger.effect.is_none() {
                self.release.right_trigger = true;
            }
            owns.left_trigger = false;
            owns.right_trigger = false;
        }
        if fb.lights == Source::Periph {
            owns.lightbar = false;
            owns.player_leds = false;
            owns.mic_led = false;
        }
        self.dirty = true;
    }

    /// Sections to write over a forwarded game report: the categories kept for Periph.
    pub fn overlay(&self, fx: &Effects, fb: &Feedback, now: Instant) -> Sections {
        self.sections(fx, now, |section| fb.periph(section))
    }

    /// Sections for Periph's own report. `game_forwarding`: emulation is on, so sections the
    /// game owns (in categories left to the game) are skipped.
    fn own(&self, fx: &Effects, fb: &Feedback, now: Instant, game_forwarding: bool) -> Sections {
        let owns = self.game_owns;
        let mut s = self.sections(fx, now, |section| {
            fb.periph(section)
                || !game_forwarding
                || !match section {
                    Section::Lightbar => owns.lightbar,
                    Section::PlayerLeds => owns.player_leds,
                    Section::MicLed => owns.mic_led,
                    Section::Left => owns.left_trigger,
                    Section::Right => owns.right_trigger,
                }
        });
        let off = TriggerEffect::Off.encode().ok();
        if self.release.left_trigger && s.left_trigger.is_none() && !(game_forwarding && owns.left_trigger) {
            s.left_trigger = off;
        }
        if self.release.right_trigger && s.right_trigger.is_none() && !(game_forwarding && owns.right_trigger) {
            s.right_trigger = off;
        }
        s
    }

    fn sections(&self, fx: &Effects, now: Instant, include: impl Fn(Section) -> bool) -> Sections {
        let mut s = Sections::default();
        if include(Section::Lightbar) {
            s.lightbar = fx.lightbar.color_at(self.t(now));
        }
        if fx.player_leds.enabled && include(Section::PlayerLeds) {
            s.player_leds = Some((fx.player_leds.mask, fx.player_leds.brightness));
        }
        if include(Section::MicLed) {
            s.mic_led = fx.mic_led.value_at(self.t(now));
        }
        if include(Section::Left) {
            s.left_trigger = fx.left_trigger.effect.as_ref().and_then(|e| e.encode().ok());
        }
        if include(Section::Right) {
            s.right_trigger = fx.right_trigger.effect.as_ref().and_then(|e| e.encode().ok());
        }
        s
    }

    /// Periph's own report, if one is due now (change, animation frame or keepalive).
    pub fn due(&mut self, fx: &Effects, fb: &Feedback, now: Instant, game_forwarding: bool) -> Option<OutputReport> {
        let since = self.last_sent.map(|at| now.duration_since(at));
        let due = self.dirty
            || since.is_none_or(|d| d >= KEEPALIVE)
            || ((fx.lightbar.animated() || fx.mic_led.mode == MicMode::Blink) && since.is_none_or(|d| d >= FRAME));
        if !due {
            return None;
        }
        let sections = self.own(fx, fb, now, game_forwarding);
        if sections.lightbar.is_some() && !self.lightbar_released {
            // Release the boot animation on its own first; the colour follows on the next poll.
            self.lightbar_released = true;
            self.last_sent = Some(now);
            return Some(Sections { release_lightbar: true, ..Default::default() }.report());
        }
        self.dirty = false;
        self.last_sent = Some(now);
        self.release = SectionFlags::default();
        (!sections.is_empty()).then(|| sections.report())
    }
}

#[derive(Clone, Copy)]
enum Section {
    Lightbar,
    PlayerLeds,
    MicLed,
    Left,
    Right,
}

#[cfg(test)]
mod tests {
    use super::*;
    use periph_core::serde_json;

    const FB: Feedback = Feedback { lights: Source::Game, haptics: true, triggers: Source::Game };

    fn weapon() -> TriggerEffect {
        TriggerEffect::Weapon { start: 2, end: 6, strength: 8 }
    }

    #[test]
    fn lightbar_colours() {
        let mut l = Lightbar { mode: LightMode::Static, color: [255, 128, 0], brightness: 50, ..Default::default() };
        assert_eq!(l.color_at(0.0), Some([128, 64, 0]));
        l.mode = LightMode::Strobe;
        l.brightness = 100;
        l.speed = 9; // period 1 s
        assert_eq!(l.color_at(0.25), Some([255, 128, 0]));
        assert_eq!(l.color_at(0.75), Some([0, 0, 0]));
        l.mode = LightMode::Breathing;
        assert_eq!(l.color_at(0.0), Some([0, 0, 0]));
        assert_eq!(l.color_at(0.5), Some([255, 128, 0]));
        l.mode = LightMode::Rainbow;
        assert_eq!(l.color_at(0.0), Some([255, 0, 0]));
        l.mode = LightMode::Off;
        assert_eq!(l.color_at(0.0), Some([0, 0, 0]), "switched off: dark, not left alone");
        l.mode = LightMode::None;
        assert_eq!(l.color_at(0.0), None);
    }

    #[test]
    fn nothing_is_sent_while_periph_controls_nothing() {
        let mut e = Engine::default();
        assert!(e.due(&Effects::default(), &FB, Instant::now(), false).is_none());
    }

    #[test]
    fn own_report_carries_controlled_sections_and_keepalive() {
        let fx = Effects {
            lightbar: Lightbar { mode: LightMode::Static, color: [1, 2, 3], ..Default::default() },
            right_trigger: Trigger { effect: Some(weapon()) },
            ..Default::default()
        };
        let mut e = Engine::default();
        let t0 = Instant::now();
        let release = e.due(&fx, &FB, t0, false).expect("dirty at start");
        assert_eq!((release.raw[2], release.raw[39], release.raw[42]), (0, 0x02, 0x02), "boot animation released on its own");
        let r = e.due(&fx, &FB, t0, false).expect("then the colour");
        assert_eq!(r.lightbar_rgb(), (1, 2, 3));
        assert_eq!(r.raw[39] & 0x02, 0, "no further releases");
        assert_eq!(r.right_trigger()[0], 0x25);
        assert!(e.due(&fx, &FB, t0 + Duration::from_millis(500), false).is_none(), "static: no resend before keepalive");
        assert!(e.due(&fx, &FB, t0 + KEEPALIVE, false).is_some(), "keepalive");
    }

    #[test]
    fn each_category_goes_to_its_source() {
        let fx = Effects {
            lightbar: Lightbar { mode: LightMode::Static, color: [9, 9, 9], ..Default::default() },
            right_trigger: Trigger { effect: Some(weapon()) },
            ..Default::default()
        };
        let mut e = Engine::default();
        e.game_owns = SectionFlags { lightbar: true, right_trigger: true, ..Default::default() };
        let now = Instant::now();
        // Both left to the game: it owns both sections, Periph sends nothing over them.
        assert!(e.due(&fx, &FB, now, true).is_none());
        assert!(e.overlay(&fx, &FB, now).is_empty());
        // Triggers kept for Periph: its weapon goes over every game report.
        let fb = Feedback { triggers: Source::Periph, ..FB };
        e.touch();
        let r = e.due(&fx, &fb, now, true).unwrap();
        assert_eq!(r.right_trigger()[0], 0x25);
        assert_eq!(r.raw[2] & 0x04, 0, "lightbar still left to the game");
        let overlay = e.overlay(&fx, &fb, now);
        assert!(overlay.right_trigger.is_some() && overlay.lightbar.is_none());
    }

    #[test]
    fn filter_drops_what_periph_keeps() {
        let mut raw = [0u8; 48];
        raw[0] = 0x02;
        raw[1] = 0x01 | 0x04 | 0x08; // rumble + both triggers
        raw[2] = 0x04; // lightbar
        raw[3] = 100;
        let game = OutputReport::parse(&raw).unwrap();

        let mut r = game;
        FB.filter(&mut r);
        assert_eq!(r, game, "everything left to the game passes untouched");

        let mut r = game;
        Feedback { lights: Source::Periph, haptics: false, ..FB }.filter(&mut r);
        assert_eq!((r.raw[1], r.raw[2], r.rumble_update()), (0x0C, 0, None));

        let mut r = game;
        Feedback::NONE.filter(&mut r);
        assert!(r.is_noop());
    }

    #[test]
    fn taking_triggers_back_resets_the_games() {
        let fx = Effects::default();
        let mut e = Engine::default();
        e.game_owns = SectionFlags { right_trigger: true, lightbar: true, ..Default::default() };
        let fb = Feedback { triggers: Source::Periph, ..FB };
        e.take_back(&fx, &fb);
        let r = e.due(&fx, &fb, Instant::now(), true).unwrap();
        assert_eq!(r.right_trigger()[0], 0x05, "back to off");
        assert_eq!(r.raw[2], 0, "the game keeps the lightbar");
        assert!(e.game_owns.lightbar && !e.game_owns.right_trigger);
    }

    #[test]
    fn sections_the_game_never_set_stay_with_periph() {
        let fx = Effects { lightbar: Lightbar { mode: LightMode::Static, color: [5, 5, 5], ..Default::default() }, ..Default::default() };
        let mut e = Engine { lightbar_released: true, ..Default::default() };
        e.game_owns = SectionFlags { right_trigger: true, ..Default::default() };
        assert_eq!(e.due(&fx, &FB, Instant::now(), true).unwrap().lightbar_rgb(), (5, 5, 5));
    }

    #[test]
    fn releasing_a_trigger_sends_off_once() {
        let fx = Effects::default();
        let mut e = Engine::default();
        let t0 = Instant::now();
        e.release_trigger(true);
        let r = e.due(&fx, &FB, t0, false).unwrap();
        assert_eq!(r.left_trigger()[0], 0x05);
        assert_eq!(r.raw[1], 0x08);
        e.touch();
        assert!(e.due(&fx, &FB, t0, false).is_none(), "released only once");
    }

    #[test]
    fn game_leaving_resets_its_triggers() {
        let fx = Effects::default();
        let mut e = Engine::default();
        e.game_owns = SectionFlags { left_trigger: true, lightbar: true, ..Default::default() };
        e.game_left(&fx);
        let r = e.due(&fx, &FB, Instant::now(), false).unwrap();
        assert_eq!(r.left_trigger()[0], 0x05);
        assert_eq!(r.raw[1], 0x08, "only the left trigger");
        assert_eq!(r.raw[2], 0, "lightbar is not reset");
    }

    #[test]
    fn animation_frames() {
        let fx = Effects { lightbar: Lightbar { mode: LightMode::Rainbow, ..Default::default() }, ..Default::default() };
        let mut e = Engine { lightbar_released: true, ..Default::default() };
        let t0 = Instant::now();
        assert!(e.due(&fx, &FB, t0, false).is_some());
        assert!(e.due(&fx, &FB, t0 + Duration::from_millis(10), false).is_none());
        assert!(e.due(&fx, &FB, t0 + FRAME, false).is_some());
    }

    #[test]
    fn mic_led_modes() {
        let mut mic = MicLed { mode: MicMode::Blink, interval_ms: 250 };
        assert_eq!((mic.value_at(0.1), mic.value_at(0.3), mic.value_at(0.6)), (Some(1), Some(0), Some(1)));
        mic.mode = MicMode::Auto;
        assert_eq!(mic.value_at(0.0), None, "never set: left alone");
        assert!(MicLed { interval_ms: 50, ..mic }.validate().is_err());
        let fx = Effects { mic_led: MicLed { mode: MicMode::Blink, interval_ms: 500 }, ..Default::default() };
        let mut e = Engine::default();
        let t0 = Instant::now();
        assert_eq!(e.due(&fx, &FB, t0, false).unwrap().mic_led(), 1);
        assert!(e.due(&fx, &FB, t0 + FRAME, false).is_some(), "blinking: sent every frame");
    }

    #[test]
    fn config_json_shape() {
        let fx: Effects = serde_json::from_str(
            r#"{"lightbar":{"mode":"breathing","color":[255,0,0],"brightness":60,"speed":3,"override":true},
                "rightTrigger":{"effect":{"kind":"bow","start":1,"end":8,"strength":5,"snap":8},"override":false}}"#,
        )
        .unwrap(); // "override" comes from older saved configs and is ignored
        assert_eq!(fx.lightbar.mode, LightMode::Breathing);
        assert!(matches!(fx.right_trigger.effect, Some(TriggerEffect::Bow { .. })));
        assert_eq!(fx.left_trigger, Trigger::default(), "missing sections default");
    }
}
