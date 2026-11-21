//! Driver contract for Periph.
//!
//! A device implementation is two pieces:
//! - a [`Driver`]: a stateless descriptor registered once at startup (id, name, how to open the
//!   device, optional tray integration);
//! - a [`Session`]: an open handle to one physical device, owned by a dedicated worker thread.
//!
//! The app runs every registered driver in its own thread with [`run`]: it opens the device
//! (retrying while absent), reads the configuration, applies UI commands, polls live status and
//! reports everything as [`Event`]s. Config, status and commands are JSON values whose shape is
//! private to the driver and its frontend module (`app/src/devices/<id>/`): the core never
//! looks inside them.
//!
//! See `docs/adding-a-device.md` for the full walkthrough.

use serde::Serialize;
use serde_json::Value;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

pub use serde_json;

pub mod i18n;
pub use i18n::{t, tf};

/// A device that is not available right now, with a stable `code` the UI can react to
/// (e.g. "asleep" to show "move the mouse to wake it"). Return it from [`Driver::open`]:
/// `return Err(Absent::new("asleep", e).into())`. Other errors are shown as plain text.
#[derive(Debug)]
pub struct Absent {
    pub code: &'static str,
    pub message: String,
}

impl Absent {
    pub fn new(code: &'static str, message: impl std::fmt::Display) -> Self {
        Absent { code, message: message.to_string() }
    }
}

impl std::fmt::Display for Absent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Absent {}

/// What kind of peripheral this is. Only used for icons and grouping in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Mouse,
    Keyboard,
    Headset,
    Controller,
    Other,
}

/// Static description of a driver. `id` must match the frontend folder `app/src/devices/<id>/`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: Kind,
}

/// Returned by [`Driver::open`] together with the session.
#[derive(Debug, Clone, Default)]
pub struct Connected {
    /// Product string shown in the UI (HID product string is fine).
    pub product: String,
    /// Serial number, or empty when the device has none.
    pub serial: String,
}

/// A device implementation, registered once in the app (`crates/periph-app/src/drivers/mod.rs`).
pub trait Driver: Send + Sync + 'static {
    fn info(&self) -> DeviceInfo;

    /// Find and open the device. `Err` means "not available right now"; its message is shown in
    /// the UI as a technical detail. Wrap known states in [`Absent`] so the UI can show a proper,
    /// translated explanation for them.
    fn open(&self) -> anyhow::Result<(Box<dyn Session>, Connected)>;

    /// How long to wait before trying [`Driver::open`] again after it failed.
    fn retry_interval(&self) -> Duration {
        Duration::from_secs(2)
    }

    /// Optional system tray integration: battery, tooltip line and the controls of the quick
    /// settings flyout (left click on the device's tray icon).
    fn tray(&self, _snap: &Snapshot) -> Option<Tray> {
        None
    }

    /// A tray item of this driver was clicked (`id` without the driver prefix). Return a command
    /// to send to the session, exactly as the frontend would.
    fn tray_command(&self, _id: &str, _snap: &Snapshot) -> Option<Value> {
        None
    }
}

/// An open device. Lives on the driver's worker thread; dropped when the device goes away.
pub trait Session: Send {
    /// Read the full configuration the UI edits. Called after connecting and after every command.
    fn read_config(&mut self) -> anyhow::Result<Value>;

    /// Apply one command from the UI (or the tray). The config is re-read afterwards.
    /// Reject anything the official software never sends: never write guessed bytes.
    fn apply(&mut self, cmd: Value) -> anyhow::Result<()>;

    /// Called whenever no command arrived for [`Session::poll_interval`]. Return live status
    /// (battery, current DPI, ...) if the device has any. `Err` means the device is gone.
    fn poll(&mut self) -> anyhow::Result<Option<Value>> {
        Ok(None)
    }

    fn poll_interval(&self) -> Duration {
        Duration::from_millis(250)
    }
}

/// Last known state of one device, kept by the app and handed to the UI on startup.
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub connected: bool,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub reason: Option<String>,
    /// [`Absent::code`] of the last disconnect, if the driver gave one.
    pub code: Option<String>,
    pub config: Option<Value>,
    pub status: Option<Value>,
}

impl Snapshot {
    pub fn update(&mut self, e: &Event) {
        match e {
            Event::Connected { product, serial } => {
                self.connected = true;
                self.product = Some(product.clone());
                self.serial = Some(serial.clone());
                self.reason = None;
                self.code = None;
            }
            Event::Disconnected { reason, code } => {
                *self = Snapshot { reason: Some(reason.clone()), code: code.clone(), ..Default::default() }
            }
            Event::Config { config } => self.config = Some(config.clone()),
            Event::Status { status } => self.status = Some(status.clone()),
            Event::Error { .. } | Event::Done => {}
        }
    }
}

/// Worker -> app/UI. Serialized as `{"type":"config","config":{...}}` etc.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    Connected { product: String, serial: String },
    Disconnected {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
    },
    Config { config: Value },
    Status { status: Value },
    Error { message: String },
    /// A command finished (success or error); the UI clears its busy state.
    Done,
}

/// Tray contribution of one device.
#[derive(Debug, Clone, Default)]
pub struct Tray {
    /// Tooltip of the device's tray icon, e.g. "VT2 Max · bateria 80%".
    pub header: String,
    /// Battery indicator for this device's tray icon and flyout: (percent, charging).
    pub battery: Option<(u8, bool)>,
    /// Quick settings in the flyout: a `Check` is a switch, a `Submenu` of `Check`s a choice
    /// group (keep its option labels short), `Colors` a row of swatches, a `Slider` a range, an
    /// `Action` a button.
    pub items: Vec<TrayItem>,
}

/// Serialized for the tray flyout as `{"type":"check","id":...,"label":...,"checked":true}` etc.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TrayItem {
    Action { id: String, label: String },
    Check { id: String, label: String, checked: bool },
    Submenu { label: String, items: Vec<TrayItem> },
    /// Colour swatches (e.g. lightbar presets); clicking one sends its `id`.
    Colors { label: String, options: Vec<TrayColor> },
    /// A range (e.g. brightness); releasing it sends `"<id>:<value>"`. `unit` is appended to the
    /// shown value ("%").
    Slider { id: String, label: String, value: u32, min: u32, max: u32, step: u32, unit: String },
    Separator,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrayColor {
    pub id: String,
    pub rgb: [u8; 3],
    pub checked: bool,
}

fn disconnected(e: &anyhow::Error) -> Event {
    Event::Disconnected { reason: e.to_string(), code: e.downcast_ref::<Absent>().map(|a| a.code.to_string()) }
}

/// Runs one driver forever (until the command channel closes), reporting through `emit`.
/// While the device stays absent, a retry that fails the same way is not reported again.
pub fn run(driver: &dyn Driver, rx: Receiver<Value>, emit: impl Fn(Event)) {
    // reason and code of the last failed open, while the device is absent
    let mut absent: Option<(String, Option<String>)> = None;
    loop {
        let (mut dev, info) = match driver.open() {
            Ok(x) => x,
            Err(e) => {
                let event = disconnected(&e);
                if let Event::Disconnected { reason, code } = &event {
                    let key = Some((reason.clone(), code.clone()));
                    if absent != key {
                        absent = key;
                        emit(event);
                    }
                }
                match rx.recv_timeout(driver.retry_interval()) {
                    Ok(_) => emit(Event::Done), // a command for an absent device: just release the UI
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                continue;
            }
        };
        absent = None;
        emit(Event::Connected { product: info.product, serial: info.serial });
        match dev.read_config() {
            Ok(config) => emit(Event::Config { config }),
            Err(e) => {
                emit(disconnected(&e));
                std::thread::sleep(driver.retry_interval());
                continue;
            }
        }
        loop {
            match rx.recv_timeout(dev.poll_interval()) {
                Ok(cmd) => {
                    if let Err(e) = dev.apply(cmd) {
                        emit(Event::Error { message: e.to_string() });
                    }
                    match dev.read_config() {
                        Ok(config) => emit(Event::Config { config }),
                        Err(e) => {
                            emit(Event::Done);
                            emit(disconnected(&e));
                            break;
                        }
                    }
                    emit(Event::Done);
                }
                Err(RecvTimeoutError::Timeout) => match dev.poll() {
                    Ok(Some(status)) => emit(Event::Status { status }),
                    Ok(None) => {}
                    Err(e) => {
                        emit(disconnected(&e));
                        break;
                    }
                },
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }
}

/// Decode a JSON command into the driver's own typed enum (usually `#[serde(tag = "type")]`).
pub fn parse<T: serde::de::DeserializeOwned>(cmd: Value) -> anyhow::Result<T> {
    serde_json::from_value(cmd).map_err(|e| anyhow::anyhow!("{}: {e}", t("errors.invalidCommand")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::mpsc::channel;
    use std::sync::{Arc, Mutex};

    /// A fake device that is absent on the first open, then echoes commands into its config.
    struct Fake {
        opens: AtomicU32,
    }
    struct FakeSession {
        value: i64,
    }
    impl Driver for Fake {
        fn info(&self) -> DeviceInfo {
            DeviceInfo { id: "fake", name: "Fake", kind: Kind::Other }
        }
        fn open(&self) -> anyhow::Result<(Box<dyn Session>, Connected)> {
            if self.opens.fetch_add(1, Ordering::SeqCst) == 0 {
                return Err(Absent::new("notFound", "not found").into());
            }
            Ok((Box::new(FakeSession { value: 1 }), Connected { product: "Fake".into(), serial: "1".into() }))
        }
        fn retry_interval(&self) -> Duration {
            Duration::from_millis(5)
        }
    }
    impl Session for FakeSession {
        fn read_config(&mut self) -> anyhow::Result<Value> {
            Ok(serde_json::json!({ "value": self.value }))
        }
        fn apply(&mut self, cmd: Value) -> anyhow::Result<()> {
            self.value = cmd["value"].as_i64().ok_or_else(|| anyhow::anyhow!("sem valor"))?;
            Ok(())
        }
        fn poll_interval(&self) -> Duration {
            Duration::from_millis(5)
        }
    }

    #[test]
    fn worker_reconnects_applies_and_reports() {
        let (tx, rx) = channel();
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev = events.clone();
        let t = std::thread::spawn(move || run(&Fake { opens: AtomicU32::new(0) }, rx, move |e| ev.lock().unwrap().push(e)));
        std::thread::sleep(Duration::from_millis(50));
        tx.send(serde_json::json!({ "value": 7 })).unwrap();
        tx.send(serde_json::json!({})).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        drop(tx);
        t.join().unwrap();

        let mut snap = Snapshot::default();
        let events = events.lock().unwrap();
        events.iter().for_each(|e| snap.update(e));
        assert!(matches!(&events[0], Event::Disconnected { code: Some(c), .. } if c == "notFound"));
        assert!(snap.connected);
        assert_eq!(snap.config, Some(serde_json::json!({ "value": 7 })));
        assert!(events.iter().any(|e| matches!(e, Event::Error { .. })));
    }

    /// Always absent, the same way every time.
    struct Gone;
    impl Driver for Gone {
        fn info(&self) -> DeviceInfo {
            DeviceInfo { id: "gone", name: "Gone", kind: Kind::Other }
        }
        fn open(&self) -> anyhow::Result<(Box<dyn Session>, Connected)> {
            Err(Absent::new("notFound", "not found").into())
        }
        fn retry_interval(&self) -> Duration {
            Duration::from_millis(2)
        }
    }

    #[test]
    fn repeated_absence_is_reported_once() {
        let (tx, rx) = channel();
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev = events.clone();
        let t = std::thread::spawn(move || run(&Gone, rx, move |e| ev.lock().unwrap().push(e)));
        std::thread::sleep(Duration::from_millis(40));
        drop(tx);
        t.join().unwrap();
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 1, "{events:?}");
        assert!(matches!(&events[0], Event::Disconnected { code: Some(c), .. } if c == "notFound"));
    }
}
