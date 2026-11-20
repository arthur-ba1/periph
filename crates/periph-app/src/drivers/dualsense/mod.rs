//! Sony DualSense driver: physical HID + virtual controller emulation through our own kernel
//! driver (`periph-vpad`, its own repository, built on the Windows Virtual HID Framework).
//!
//! Data path when emulation is on:
//!   physical pad ──input 0x01──▶ forwarder thread ──▶ virtual device (games)
//!   games ──output 0x02──▶ virtual device ──▶ poll() ──▶ physical pad (if forwarding)
//!
//! The pad may be on USB or Bluetooth. Everything above the transport works on USB reports;
//! [`Transport`] converts to and from the Bluetooth framing (0x31 + CRC) at the HID handle.
//!
//! Input is forwarded by a dedicated thread blocked on the pad's HID handle, so latency does not
//! depend on the worker's poll timer (Windows timers tick at ~15 ms by default). The feature
//! responses the virtual device answers come from the physical controller itself (GET_FEATURE
//! only). While emulating, HidHide (if installed) hides the physical pad from everything but
//! Periph, so games see exactly one controller. Frontend contract: `docs/dualsense-protocol.md`.

mod effects;

use anyhow::{Context, Result};
use dualsense_proto::bluetooth;
use dualsense_proto::feature::{FeatureResponse, FeatureSet};
use dualsense_proto::input::InputReport;
use dualsense_proto::effects::SectionFlags;
use dualsense_proto::output::OutputReport;
use dualsense_proto::{PID, VID};
use dualsense_vpad::Channel;
use hidapi::{HidApi, HidDevice};
use periph_core::{self as core, serde_json::{self, Value}, t, Absent, Connected, DeviceInfo, Driver, Kind, Session};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Same policy as the vendor app: stop the motors if no rumble update arrived for 5 s.
const RUMBLE_WATCHDOG: Duration = Duration::from_secs(5);
/// Minimum spacing of status events sent to the UI (unless something visible changed).
const STATUS_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Cmd {
    Refresh,
    Emulation(bool),
    ForwardFeedback(bool),
    HidePhysical(bool),
    Lightbar(effects::Lightbar),
    PlayerLeds(effects::PlayerLeds),
    MicLed(effects::MicLed),
    LeftTrigger(effects::Trigger),
    RightTrigger(effects::Trigger),
}

/// Driver config. Everything but `emulation` is saved to disk and restored on connect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub emulation: bool,
    pub forward_feedback: bool,
    /// Hide the physical pad with HidHide while emulating (only if HidHide is installed).
    pub hide_physical: bool,
    #[serde(flatten)]
    pub effects: effects::Effects,
}

impl Default for Config {
    fn default() -> Self {
        Self { emulation: false, forward_feedback: true, hide_physical: true, effects: effects::Effects::default() }
    }
}

fn config_file() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Periph").join("dualsense.json")
}

/// Saved config (emulation always starts off: it needs the driver and a deliberate click).
fn load_config() -> Config {
    let mut c: Config = std::fs::read_to_string(config_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    c.emulation = false;
    // Drop anything a hand-edited file could hold outside the allowed ranges.
    let fx = &mut c.effects;
    if fx.lightbar.validate().is_err() {
        fx.lightbar = effects::Lightbar::default();
    }
    if fx.player_leds.validate().is_err() {
        fx.player_leds = effects::PlayerLeds::default();
    }
    if fx.left_trigger.validate().is_err() {
        fx.left_trigger = effects::Trigger::default();
    }
    if fx.right_trigger.validate().is_err() {
        fx.right_trigger = effects::Trigger::default();
    }
    c
}

fn save_config(c: &Config) {
    let path = config_file();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(c) {
        let _ = std::fs::write(path, json);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HidHideState {
    /// HidHide is not installed: games see the physical and the virtual pad.
    Missing,
    /// Installed, the physical pad is visible.
    Off,
    /// The physical pad is hidden from everything but Periph.
    Hiding,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub virtual_active: bool,
    pub battery_raw: u8,
    pub output_reports: u64,
    pub hid_hide: HidHideState,
}

// ---------------------------------------------------------------------------------------------
// Physical vs virtual: both have the same VID/PID and USB-shaped instance ids.

/// Whether a HID device path belongs to an emulated controller rather than real hardware.
///
/// Structure is read from the PnP tree: a physical pad descends from a PCI device, while an
/// emulated one descends from a root-enumerated software bus (`ROOT\...`). Fast path markers
/// catch our own device without a configuration-manager round trip.
fn is_virtual_path(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    if p.contains("periph") || p.contains("vhf") {
        return true;
    }
    match instance_id_from_hid_path(&p) {
        // Unknown = not on a real PCI-backed USB tree, so not safe to treat as physical.
        Some(id) => !descends_from_pci(&id),
        None => true,
    }
}

/// Whether a HID device path is a pad paired over Bluetooth (classic HID service UUID, or the
/// Bluetooth enumerators).
fn is_bluetooth_path(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    p.contains("00001124-0000-1000-8000-00805f9b34fb") || p.contains("bthenum") || p.contains("bthledevice")
}

/// `\\?\hid#vid_054c&pid_0ce6&mi_03#9&1a5951d8&0&0003#{guid}` ->
/// `HID\VID_054C&PID_0CE6&MI_03\9&1A5951D8&0&0003`
fn instance_id_from_hid_path(path: &str) -> Option<String> {
    let body = path.strip_prefix(r"\\?\")?;
    // The interface GUID is the last `#{...}`; Bluetooth hardware ids contain a `{...}` too.
    let body = body.rsplit_once("#{").map_or(body, |(b, _)| b);
    let parts: Vec<&str> = body.split('#').collect();
    match parts.as_slice() {
        [enumerator, id, instance, ..] => Some(format!("{enumerator}\\{id}\\{instance}").to_ascii_uppercase()),
        _ => None,
    }
}

#[cfg(windows)]
mod pnp {
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_Device_IDW, CM_Get_Parent, CM_Locate_DevNodeW, CM_LOCATE_DEVNODE_NORMAL,
    };

    pub fn locate(instance_id: &str) -> Option<u32> {
        let wide: Vec<u16> = instance_id.encode_utf16().chain(std::iter::once(0)).collect();
        let mut node = 0u32;
        (unsafe { CM_Locate_DevNodeW(&mut node, wide.as_ptr(), CM_LOCATE_DEVNODE_NORMAL) } == 0).then_some(node)
    }

    pub fn id_of(node: u32) -> Option<String> {
        let mut buf = [0u16; 256];
        let rc = unsafe { CM_Get_Device_IDW(node, buf.as_mut_ptr(), buf.len() as u32, 0) };
        if rc != 0 {
            return None;
        }
        let end = buf.iter().position(|&w| w == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..end]))
    }

    pub fn parent(node: u32) -> Option<u32> {
        let mut parent = 0u32;
        (unsafe { CM_Get_Parent(&mut parent, node, 0) } == 0).then_some(parent)
    }
}

/// Walks the PnP parent chain: true when a PCI ancestor is reached before any root bus.
#[cfg(windows)]
fn descends_from_pci(instance_id: &str) -> bool {
    let Some(mut node) = pnp::locate(instance_id) else {
        return false;
    };
    for _ in 0..12 {
        if let Some(id) = pnp::id_of(node).map(|s| s.to_ascii_uppercase()) {
            if id.starts_with("PCI\\") {
                return true;
            }
            if id.starts_with(r"ROOT\") {
                return false; // software bus: virtual device
            }
        }
        let Some(parent) = pnp::parent(node) else {
            return false;
        };
        node = parent;
    }
    false
}

#[cfg(not(windows))]
fn descends_from_pci(_instance_id: &str) -> bool {
    false
}

/// The instance id exactly as the configuration manager spells it (what HidHide matches).
#[cfg(windows)]
fn canonical_instance_id(instance_id: &str) -> String {
    pnp::locate(instance_id).and_then(pnp::id_of).unwrap_or_else(|| instance_id.to_string())
}

#[cfg(not(windows))]
fn canonical_instance_id(instance_id: &str) -> String {
    instance_id.to_string()
}

// ---------------------------------------------------------------------------------------------
// Physical pad

fn tf_hidden(e: impl std::fmt::Display) -> String {
    format!("{} ({e})", t("dualsense.errors.hidden"))
}

/// How the physical pad is connected. Bluetooth carries the USB report bodies in other report
/// ids with a CRC (see `dualsense_proto::bluetooth`); this converts at the HID handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transport {
    Usb,
    Bluetooth,
}

/// Read buffer: Windows pads Bluetooth reads to the descriptor's largest input report (547).
const READ_BUF: usize = 1024;
/// Feature reads over Bluetooth need a buffer at least as large as the largest feature report.
const FEATURE_BUF_BT: usize = 1024;

impl Transport {
    fn of_path(path: &str) -> Self {
        if is_bluetooth_path(path) { Self::Bluetooth } else { Self::Usb }
    }

    /// One raw read → a USB input report, or `None` for reports we do not use (the reduced
    /// Bluetooth report sent before full mode is enabled, CRC failures).
    fn input(self, bytes: &[u8]) -> Option<InputReport> {
        match self {
            Self::Usb => InputReport::parse(bytes.get(..dualsense_proto::INPUT_REPORT_LEN)?).ok(),
            Self::Bluetooth => bluetooth::parse_input(bytes).ok(),
        }
    }

    fn write(self, dev: &HidDevice, report: &OutputReport) -> Result<()> {
        match self {
            Self::Usb => dev.write(report.as_bytes()),
            Self::Bluetooth => dev.write(&bluetooth::output_report(report)),
        }
        .context("write to the controller")?;
        Ok(())
    }

    /// GET_FEATURE `id` (read-only), returned in its USB form.
    fn read_feature(self, dev: &HidDevice, id: u8, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; if self == Self::Bluetooth { FEATURE_BUF_BT } else { len }];
        buf[0] = id;
        let n = dev.get_feature_report(&mut buf).context("read controller feature report")?;
        match self {
            Self::Usb => Ok(buf[..n.min(len)].to_vec()),
            Self::Bluetooth => bluetooth::feature_to_usb(&buf[..n], len).map_err(anyhow::Error::msg),
        }
    }
}

/// A physical DualSense: one HID handle for output, feature reads and status input.
/// The emulation forwarder opens its own second handle to the same path.
struct PhysicalPad {
    dev: HidDevice,
    path: CString,
    transport: Transport,
    instance_id: Option<String>,
    product: String,
}

impl PhysicalPad {
    /// Opens the *physical* pad only (virtual pads share its VID/PID, see [`is_virtual_path`]).
    fn open() -> Result<Self> {
        let api = HidApi::new()?;
        let mut saw_virtual = false;
        let mut candidates: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == 0x01 && d.usage() == 0x05)
            .collect();
        // A cable beats a radio: prefer USB when the pad shows up on both.
        candidates.sort_by_key(|d| Transport::of_path(&d.path().to_string_lossy()) == Transport::Bluetooth);
        for d in candidates {
            let path = d.path().to_string_lossy().into_owned();
            let transport = Transport::of_path(&path);
            // Emulated pads are USB-shaped; a Bluetooth pad hangs off the real radio.
            if transport == Transport::Usb && is_virtual_path(&path) {
                saw_virtual = true;
                continue;
            }
            let dev = match d.open_device(&api) {
                Ok(dev) => dev,
                // Typical cause: another app left this pad in HidHide's blacklist
                // without whitelisting Periph.
                Err(e) if hidhide::installed() => return Err(Absent::new("hidden", tf_hidden(e)).into()),
                Err(e) => return Err(anyhow::anyhow!(e).context("open the controller")),
            };
            dev.set_blocking_mode(false)?;
            if transport == Transport::Bluetooth {
                // Reading the calibration report switches the pad from its reduced input report
                // to the full one (the official software and Linux do the same on connect).
                use dualsense_proto::feature::{CALIBRATION_ID, CALIBRATION_LEN};
                transport.read_feature(&dev, CALIBRATION_ID, CALIBRATION_LEN).context("enable full input reports")?;
            }
            return Ok(Self {
                dev,
                path: d.path().to_owned(),
                transport,
                instance_id: instance_id_from_hid_path(&path).map(|id| canonical_instance_id(&id)),
                product: d.product_string().unwrap_or("DualSense").to_string(),
            });
        }
        if saw_virtual {
            return Err(Absent::new("emulatorRunning", t("dualsense.errors.emulatorRunning")).into());
        }
        Err(Absent::new("notFound", t("dualsense.errors.notFound")).into())
    }

    /// Drains pending input on the status handle; returns the newest report, if any.
    fn drain_input(&mut self) -> Result<Option<InputReport>> {
        let mut latest = None;
        let mut buf = [0u8; READ_BUF];
        loop {
            let n = self.dev.read(&mut buf).map_err(|e| anyhow::anyhow!(e).context(t("dualsense.errors.disconnected")))?;
            if n == 0 {
                return Ok(latest);
            }
            if let Some(report) = self.transport.input(&buf[..n]) {
                latest = Some(report);
            }
        }
    }

    fn write_output(&mut self, report: &OutputReport) -> Result<()> {
        self.transport.write(&self.dev, report)
    }

    /// Feature responses for the virtual (USB) device, in USB form whatever the transport.
    fn read_features(&mut self) -> Result<FeatureSet> {
        let mut set = FeatureSet::default();
        for (id, len) in dualsense_proto::feature::ALLOWED {
            let bytes = self.transport.read_feature(&self.dev, *id, *len)?;
            if let Ok(resp) = FeatureResponse::new(*id, &bytes) {
                set.insert(resp);
            }
        }
        Ok(set)
    }
}

// ---------------------------------------------------------------------------------------------
// Pure helpers (unit-tested)

/// Tracks whether the game left the motors running, to stop them after [`RUMBLE_WATCHDOG`].
#[derive(Debug, Default)]
struct RumbleWatchdog {
    since: Option<Instant>,
}

impl RumbleWatchdog {
    fn observe(&mut self, report: &OutputReport, now: Instant) {
        match report.rumble_update() {
            Some(true) => self.since = Some(now),
            Some(false) => self.since = None,
            None => {} // report does not touch the motors
        }
    }

    /// True once when the motors have been running without an update for too long.
    fn expired(&mut self, now: Instant) -> bool {
        match self.since {
            Some(at) if now.duration_since(at) >= RUMBLE_WATCHDOG => {
                self.since = None;
                true
            }
            _ => false,
        }
    }

    /// Whether the motors are believed to be running; clears the state.
    fn take_running(&mut self) -> bool {
        self.since.take().is_some()
    }
}

/// Limits status events to visible changes, plus counters at most every [`STATUS_INTERVAL`].
#[derive(Debug, Default)]
struct StatusThrottle {
    last: Option<(Instant, Status)>,
}

impl StatusThrottle {
    fn offer(&mut self, status: Status, now: Instant) -> Option<Status> {
        let send = match &self.last {
            None => true,
            Some((at, prev)) => {
                prev.virtual_active != status.virtual_active
                    || prev.hid_hide != status.hid_hide
                    || (prev != &status && now.duration_since(*at) >= STATUS_INTERVAL)
            }
        };
        if send {
            self.last = Some((now, status.clone()));
            Some(status)
        } else {
            None
        }
    }
}

fn hidhide_ledger() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Periph").join("hidhide-dualsense.txt")
}

// ---------------------------------------------------------------------------------------------
// Session

/// Opens the kernel channel. Implemented for the real Windows channel and for the mock.
pub trait SessionChannel: Channel + Sized + 'static {
    fn open_channel() -> Result<Self>;
}

impl SessionChannel for dualsense_vpad::MockChannel {
    fn open_channel() -> Result<Self> {
        Ok(Self::default())
    }
}

#[cfg(windows)]
impl SessionChannel for dualsense_vpad::windows_channel::WindowsChannel {
    fn open_channel() -> Result<Self> {
        Self::open()
    }
}

/// A running virtual pad: the channel plus the input forwarder thread.
struct Emulation<C: SessionChannel> {
    channel: Arc<Mutex<C>>,
    stop: Arc<AtomicBool>,
    forwarder: Option<JoinHandle<()>>,
    /// Set by the forwarder when it exits on an error (pad unplugged, driver gone).
    failure: Arc<Mutex<Option<String>>>,
}

impl<C: SessionChannel> Emulation<C> {
    fn start(pad: &mut PhysicalPad) -> Result<Self> {
        let mut ch = C::open_channel().context(t("dualsense.errors.driverMissing"))?;
        // Feature responses come from the physical pad itself (GET_FEATURE, read-only) and are
        // uploaded before the device exists, so a client can never see it without them.
        let features = pad.read_features()?;
        for (id, _) in dualsense_proto::feature::ALLOWED {
            if let Some(bytes) = features.get(*id) {
                ch.set_feature_response(&FeatureResponse::new(*id, bytes).map_err(anyhow::Error::msg)?)?;
            }
        }
        // Second handle for the forwarder, opened before the virtual pad exists so a failure
        // leaves nothing behind.
        let input = HidApi::new()
            .and_then(|api| api.open_path(&pad.path))
            .context("open the controller for input forwarding")?;
        ch.start()?;

        let channel = Arc::new(Mutex::new(ch));
        let stop = Arc::new(AtomicBool::new(false));
        let failure = Arc::new(Mutex::new(None));
        let transport = pad.transport;
        let spawned = {
            let (channel, stop, failure) = (channel.clone(), stop.clone(), failure.clone());
            std::thread::Builder::new().name("dualsense-input".into()).spawn(move || {
                if let Err(e) = forward_input(&input, transport, &channel, &stop) {
                    *failure.lock().unwrap() = Some(format!("{e:#}"));
                }
            })
        };
        // On spawn failure `Self` is never built, so destroy the virtual pad by hand.
        let forwarder = match spawned {
            Ok(handle) => handle,
            Err(e) => {
                let _ = channel.lock().unwrap().destroy();
                return Err(e).context("spawn input forwarder");
            }
        };
        Ok(Self { channel, stop, forwarder: Some(forwarder), failure })
    }

    fn failure(&self) -> Option<String> {
        self.failure.lock().unwrap().clone()
    }
}

impl<C: SessionChannel> Drop for Emulation<C> {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.forwarder.take() {
            let _ = thread.join();
        }
        if let Ok(mut ch) = self.channel.lock() {
            let _ = ch.destroy();
        }
    }
}

/// Forwarder loop: blocks on the pad (100 ms timeout to notice `stop`) and submits every input
/// report to the virtual device as soon as it arrives.
fn forward_input<C: Channel>(dev: &HidDevice, transport: Transport, channel: &Mutex<C>, stop: &AtomicBool) -> Result<()> {
    let mut buf = [0u8; READ_BUF];
    while !stop.load(Ordering::Relaxed) {
        let n = dev.read_timeout(&mut buf, 100).context("read the controller")?;
        if n == 0 {
            continue;
        }
        if let Some(report) = transport.input(&buf[..n]) {
            channel.lock().unwrap().submit_input(&report)?;
        }
    }
    Ok(())
}

struct DualSenseSession<C: SessionChannel> {
    pad: PhysicalPad,
    emulation: Option<Emulation<C>>,
    config: Config,
    hidhide: HidHideState,
    /// What we added to HidHide's lists while hiding (undone exactly on stop).
    hidden: Option<hidhide::Applied>,
    watchdog: RumbleWatchdog,
    throttle: StatusThrottle,
    battery_raw: u8,
    output_count: u64,
    /// Periph's own lightbar / LED / trigger effects and how they share the pad with games.
    fx: effects::Engine,
}

impl<C: SessionChannel> DualSenseSession<C> {
    fn new(pad: PhysicalPad) -> Self {
        Self {
            pad,
            emulation: None,
            config: load_config(),
            hidhide: if hidhide::installed() { HidHideState::Off } else { HidHideState::Missing },
            hidden: None,
            watchdog: RumbleWatchdog::default(),
            throttle: StatusThrottle::default(),
            battery_raw: 0,
            output_count: 0,
            fx: effects::Engine::default(),
        }
    }

    /// Emulation is on and the game's feedback reaches the pad (so the game can own sections).
    fn game_forwarding(&self) -> bool {
        self.emulation.is_some() && self.config.forward_feedback
    }

    /// Replaces one trigger's setting; a trigger Periph stops controlling goes back to off.
    fn set_trigger(&mut self, left: bool, value: effects::Trigger) -> Result<()> {
        value.validate()?;
        let slot = if left { &mut self.config.effects.left_trigger } else { &mut self.config.effects.right_trigger };
        let released = slot.effect.is_some() && value.effect.is_none();
        *slot = value;
        if released {
            self.fx.release_trigger(left);
        }
        Ok(())
    }

    fn start_emulation(&mut self) -> Result<()> {
        if self.emulation.is_none() {
            self.emulation = Some(Emulation::start(&mut self.pad)?);
        }
        Ok(())
    }

    /// Removes the virtual pad, makes the physical one visible again and stops leftover rumble.
    fn stop_emulation(&mut self) {
        // Virtual pad first, then unhide: games briefly see no pad rather than two.
        let was_emulating = self.emulation.take().is_some();
        self.unhide();
        if self.watchdog.take_running() && self.config.forward_feedback {
            let _ = self.pad.write_output(&OutputReport::rumble_stop());
        }
        if was_emulating {
            // Periph takes its sections back; triggers the game left set go back to off.
            self.fx.game_left(&self.config.effects);
        }
    }

    fn hide(&mut self) -> Result<()> {
        if self.hidden.is_some() || self.hidhide == HidHideState::Missing {
            return Ok(());
        }
        let Some(device) = self.pad.instance_id.clone() else {
            anyhow::bail!(t("dualsense.errors.hidHide"));
        };
        let exe = std::env::current_exe()?;
        match hidhide::hide(&device, &exe, &hidhide_ledger()) {
            Ok(Some(applied)) => {
                self.hidden = Some(applied);
                self.hidhide = HidHideState::Hiding;
                Ok(())
            }
            Ok(None) => {
                self.hidhide = HidHideState::Missing;
                Ok(())
            }
            Err(e) => Err(e.context(t("dualsense.errors.hidHide"))),
        }
    }

    fn unhide(&mut self) {
        if let Some(applied) = self.hidden.take() {
            let _ = hidhide::unhide(&applied, &hidhide_ledger());
            self.hidhide = HidHideState::Off;
        }
    }
}

impl<C: SessionChannel> Drop for DualSenseSession<C> {
    /// Session ends (pad unplugged, read error, app closing): never leave a frozen virtual pad,
    /// a hidden physical pad or running motors behind.
    fn drop(&mut self) {
        self.stop_emulation();
    }
}

impl<C: SessionChannel> Session for DualSenseSession<C> {
    fn read_config(&mut self) -> Result<Value> {
        Ok(serde_json::to_value(&self.config)?)
    }

    fn apply(&mut self, cmd: Value) -> Result<()> {
        match core::parse::<Cmd>(cmd)? {
            Cmd::Refresh => {}
            Cmd::Emulation(true) => {
                self.start_emulation()?;
                self.config.emulation = true;
                if self.config.hide_physical {
                    self.hide()?;
                }
            }
            Cmd::Emulation(false) => {
                self.stop_emulation();
                self.config.emulation = false;
            }
            Cmd::ForwardFeedback(on) => {
                if !on && self.watchdog.take_running() {
                    self.pad.write_output(&OutputReport::rumble_stop())?;
                }
                self.config.forward_feedback = on;
            }
            Cmd::HidePhysical(on) => {
                self.config.hide_physical = on;
                if !on {
                    self.unhide();
                } else if self.emulation.is_some() {
                    self.hide()?;
                }
            }
            Cmd::Lightbar(v) => {
                v.validate()?;
                self.config.effects.lightbar = v;
            }
            Cmd::PlayerLeds(v) => {
                v.validate()?;
                self.config.effects.player_leds = v;
            }
            Cmd::MicLed(v) => self.config.effects.mic_led = v,
            Cmd::LeftTrigger(v) => self.set_trigger(true, v)?,
            Cmd::RightTrigger(v) => self.set_trigger(false, v)?,
        }
        self.fx.touch();
        save_config(&self.config);
        Ok(())
    }

    fn poll(&mut self) -> Result<Option<Value>> {
        // Status handle: battery, and an error here means the pad is gone.
        if let Some(report) = self.pad.drain_input()? {
            self.battery_raw = report.battery_raw();
        }

        let now = Instant::now();
        let mut virtual_active = false;
        if let Some(emu) = &self.emulation {
            if let Some(failure) = emu.failure() {
                anyhow::bail!(failure);
            }
            // Game -> physical: drain the queued output reports in order.
            let mut outputs = Vec::new();
            {
                let mut ch = emu.channel.lock().unwrap();
                while let Some(report) = ch.read_output()? {
                    outputs.push(report);
                }
                virtual_active = ch.is_active()?;
            }
            let overlay = self.fx.overlay(&self.config.effects, now);
            for mut report in outputs {
                self.output_count += 1;
                self.watchdog.observe(&report, now);
                if self.config.forward_feedback {
                    // The game owns what it sets; overridden sections get Periph's value instead.
                    self.fx.game_owns = self.fx.game_owns.union(SectionFlags::of(&report));
                    overlay.apply(&mut report);
                    self.pad.write_output(&report)?;
                }
            }
        }
        let forwarding = self.game_forwarding();
        if let Some(report) = self.fx.due(&self.config.effects, now, forwarding) {
            self.pad.write_output(&report)?;
        }
        if self.watchdog.expired(now) && self.config.forward_feedback {
            self.pad.write_output(&OutputReport::rumble_stop())?;
        }

        let status = Status {
            virtual_active,
            battery_raw: self.battery_raw,
            output_reports: self.output_count,
            hid_hide: self.hidhide,
        };
        Ok(self.throttle.offer(status, now).map(serde_json::to_value).transpose()?)
    }

    fn poll_interval(&self) -> Duration {
        // Output (rumble/LEDs) is drained here; input has its own thread.
        if self.emulation.is_some() {
            Duration::from_millis(4)
        } else if self.config.effects.lightbar.animated() {
            effects::FRAME
        } else {
            Duration::from_millis(100)
        }
    }
}

pub struct DualSenseDriver;

impl Driver for DualSenseDriver {
    fn info(&self) -> DeviceInfo {
        DeviceInfo { id: "dualsense", name: "DualSense", kind: Kind::Controller }
    }

    fn open(&self) -> Result<(Box<dyn Session>, Connected)> {
        // Undo HidHide changes left by a previous run that crashed (no-op without a ledger).
        let _ = hidhide::recover(&hidhide_ledger());
        let pad = PhysicalPad::open()?;
        let product = pad.product.clone();
        #[cfg(windows)]
        let session: Box<dyn Session> = Box::new(DualSenseSession::<dualsense_vpad::windows_channel::WindowsChannel>::new(pad));
        #[cfg(not(windows))]
        let session: Box<dyn Session> = Box::new(DualSenseSession::<dualsense_vpad::MockChannel>::new(pad));
        Ok((session, Connected { product, serial: String::new() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(flags0: u8, right: u8, left: u8) -> OutputReport {
        let mut raw = [0u8; dualsense_proto::OUTPUT_REPORT_LEN];
        raw[0] = 0x02;
        raw[1] = flags0;
        raw[3] = right;
        raw[4] = left;
        OutputReport::parse(&raw).unwrap()
    }

    #[test]
    fn hid_path_to_instance_id() {
        // Real path seen on the development machine (physical pad).
        let physical = r"\\?\hid#vid_054c&pid_0ce6&mi_03#9&1a5951d8&0&0003#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        assert_eq!(
            instance_id_from_hid_path(physical).as_deref(),
            Some(r"HID\VID_054C&PID_0CE6&MI_03\9&1A5951D8&0&0003")
        );
        assert_eq!(instance_id_from_hid_path(r"\\?\hid#vid_054c#1&2&3#{guid}").as_deref(), Some(r"HID\VID_054C\1&2&3"));
        assert!(instance_id_from_hid_path(r"\\?\hid#vid_054c").is_none());
        assert!(instance_id_from_hid_path("not a hid path").is_none());
    }

    #[test]
    fn bluetooth_pads_are_recognised() {
        // Classic Bluetooth HID path shape (HID service UUID in the hardware id).
        let bt = r"\\?\hid#{00001124-0000-1000-8000-00805f9b34fb}_vid&0002054c_pid&0ce6#8&2a6b1f3c&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        assert!(is_bluetooth_path(bt));
        assert_eq!(Transport::of_path(bt), Transport::Bluetooth);
        // Path the official software opened on the development machine.
        let seen = r"\\?\hid#{00001124-0000-1000-8000-00805f9b34fb}_vid&0002054c_pid&0ce6#9&b36f245&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
        assert_eq!(Transport::of_path(seen), Transport::Bluetooth);
        assert_eq!(
            instance_id_from_hid_path(seen).as_deref(),
            Some(r"HID\{00001124-0000-1000-8000-00805F9B34FB}_VID&0002054C_PID&0CE6\9&B36F245&0&0000")
        );
        assert!(!is_bluetooth_path(r"\\?\hid#vid_054c&pid_0ce6&mi_03#9&1a5951d8&0&0003#{4d1e55b2-f16f-11cf-88cb-001111000030}"));
    }

    #[test]
    fn our_own_virtual_device_is_rejected_without_a_pnp_call() {
        assert!(is_virtual_path(r"\\?\hid#vid_054c&pid_0ce6&mi_03#5&periph#{}"));
        assert!(is_virtual_path(r"\\?\hid#vid_054c&pid_0ce6&mi_03#5&vhf#{}"));
    }

    #[test]
    fn commands_parse_from_ui_json() {
        use serde_json::json;
        assert!(matches!(core::parse::<Cmd>(json!({"type":"refresh"})).unwrap(), Cmd::Refresh));
        assert!(matches!(core::parse::<Cmd>(json!({"type":"emulation","value":true})).unwrap(), Cmd::Emulation(true)));
        assert!(matches!(core::parse::<Cmd>(json!({"type":"forwardFeedback","value":false})).unwrap(), Cmd::ForwardFeedback(false)));
        assert!(matches!(core::parse::<Cmd>(json!({"type":"hidePhysical","value":true})).unwrap(), Cmd::HidePhysical(true)));
        assert!(core::parse::<Cmd>(json!({"type":"bogus"})).is_err());
        let lb = core::parse::<Cmd>(json!({"type":"lightbar","value":{"mode":"static","color":[1,2,3],"brightness":50,"speed":5,"override":false}})).unwrap();
        assert!(matches!(lb, Cmd::Lightbar(l) if l.color == [1, 2, 3]));
        let rt = core::parse::<Cmd>(json!({"type":"rightTrigger","value":{"effect":{"kind":"weapon","start":2,"end":6,"strength":8},"override":true}})).unwrap();
        assert!(matches!(rt, Cmd::RightTrigger(t) if t.override_game && t.effect.is_some()));
        let off = core::parse::<Cmd>(json!({"type":"leftTrigger","value":{"effect":null,"override":false}})).unwrap();
        assert!(matches!(off, Cmd::LeftTrigger(t) if t.effect.is_none()));
    }

    #[test]
    fn watchdog_fires_once_after_silence() {
        let t0 = Instant::now();
        let mut w = RumbleWatchdog::default();
        w.observe(&report(0x01, 200, 0), t0);
        assert!(!w.expired(t0 + Duration::from_secs(4)));
        assert!(w.expired(t0 + RUMBLE_WATCHDOG));
        assert!(!w.expired(t0 + Duration::from_secs(10)), "fires only once");
    }

    #[test]
    fn watchdog_ignores_reports_that_do_not_touch_the_motors() {
        let t0 = Instant::now();
        let mut w = RumbleWatchdog::default();
        w.observe(&report(0x01, 200, 0), t0);
        // A lightbar-only update later must neither restart nor cancel the timer.
        w.observe(&report(0x00, 0, 0), t0 + Duration::from_secs(3));
        assert!(w.expired(t0 + RUMBLE_WATCHDOG));
    }

    #[test]
    fn watchdog_is_cancelled_when_the_game_stops_the_motors() {
        let t0 = Instant::now();
        let mut w = RumbleWatchdog::default();
        w.observe(&report(0x01, 200, 0), t0);
        w.observe(&report(0x01, 0, 0), t0 + Duration::from_secs(1));
        assert!(!w.expired(t0 + Duration::from_secs(60)));
        assert!(!w.take_running());
    }

    #[test]
    fn status_is_throttled_except_for_visible_changes() {
        let t0 = Instant::now();
        let s = |active, reports| Status { virtual_active: active, battery_raw: 5, output_reports: reports, hid_hide: HidHideState::Off };
        let mut th = StatusThrottle::default();
        assert!(th.offer(s(false, 0), t0).is_some(), "first status always goes out");
        assert!(th.offer(s(false, 0), t0 + Duration::from_secs(5)).is_none(), "unchanged");
        assert!(th.offer(s(false, 1), t0 + Duration::from_secs(5)).is_some());
        assert!(th.offer(s(false, 2), t0 + Duration::from_millis(5010)).is_none(), "counter change, too soon");
        assert!(th.offer(s(true, 2), t0 + Duration::from_millis(5020)).is_some(), "visible change, at once");
    }
}
