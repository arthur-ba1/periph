//! <Brand Model> driver (USB VID:PID). Protocol: mydevice-proto. Frontend: app/src/devices/mydevice/.
//!
//! TEMPLATE: rename `mydevice`/`MyDevice`, then replace the example setting (polling rate) with
//! the device's real settings. Copy the shape of crates/periph-app/src/drivers/vt2.rs for a full
//! example with status polling and a tray menu.

use hidapi::{HidApi, HidDevice};
use mydevice_proto::{self as p, addr, Resp};
use periph_core::{self as core, serde_json::{self, Value}, Connected, DeviceInfo, Driver, Kind, Session};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Commands from the UI: `{ "type": "polling", "value": 1000 }`. Keep in sync with the module's store.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Cmd {
    Refresh,
    Polling(u32),
}

/// Everything the UI edits. Read in full after connecting and after every command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub polling: u32,
}

// ---- transport ----

struct Dev {
    cmd: HidDevice,
}

impl Dev {
    fn open() -> anyhow::Result<(Self, Connected)> {
        let api = HidApi::new()?;
        for &pid in &p::PIDS {
            if let Some(info) = api
                .device_list()
                .find(|d| d.vendor_id() == p::VID && d.product_id() == pid && d.usage_page() == p::CMD_USAGE_PAGE)
            {
                let cmd = info.open_device(&api)?;
                let c = Connected {
                    product: info.product_string().unwrap_or("").to_string(),
                    serial: info.serial_number().unwrap_or("").to_string(),
                };
                return Ok((Dev { cmd }, c));
            }
        }
        anyhow::bail!("não encontrado")
    }

    /// Send a command and wait for the ready answer, the way the official app does.
    fn transact(&self, out: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.cmd.write(out)?;
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut buf = [0u8; p::CMD_REPORT_LEN];
        loop {
            buf[0] = p::CMD_REPORT_ID;
            let n = self.cmd.get_input_report(&mut buf)?;
            match p::parse_response(&buf[..n])? {
                Resp::Ready(data) => return Ok(data.to_vec()),
                Resp::Busy if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(2)),
                Resp::Busy => anyhow::bail!("o dispositivo não respondeu"),
            }
        }
    }

    fn read(&self, address: u16, len: u8) -> anyhow::Result<Vec<u8>> {
        let mut data = self.transact(&p::read_cmd(address, len))?;
        data.truncate(len as usize);
        Ok(data)
    }

    fn write(&self, address: u16, data: &[u8]) -> anyhow::Result<()> {
        self.transact(&p::write_cmd(address, data)).map(|_| ())
    }
}

// ---- driver ----

fn read_config(d: &Dev) -> anyhow::Result<Config> {
    Ok(Config { polling: p::polling_from_code(d.read(addr::POLLING, 1)?[0])? })
}

fn apply(d: &Dev, cmd: Cmd) -> anyhow::Result<()> {
    match cmd {
        Cmd::Refresh => Ok(()),
        // the proto crate only encodes values the official app offers
        Cmd::Polling(hz) => d.write(addr::POLLING, &[p::polling_code(hz)?]),
    }
}

struct MyDeviceSession(Dev);

impl Session for MyDeviceSession {
    fn read_config(&mut self) -> anyhow::Result<Value> {
        Ok(serde_json::to_value(read_config(&self.0)?)?)
    }
    fn apply(&mut self, cmd: Value) -> anyhow::Result<()> {
        apply(&self.0, core::parse(cmd)?)
    }
    // fn poll(&mut self) -> anyhow::Result<Option<Value>> { ... } // battery/live status, if the device reports it
}

pub struct MyDeviceDriver;

impl Driver for MyDeviceDriver {
    fn info(&self) -> DeviceInfo {
        DeviceInfo { id: "mydevice", name: "Brand Model", kind: Kind::Mouse }
    }
    fn open(&self) -> anyhow::Result<(Box<dyn Session>, Connected)> {
        let (dev, info) = Dev::open()?;
        Ok((Box::new(MyDeviceSession(dev)), info))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The JSON the frontend sends must decode into Cmd.
    #[test]
    fn ui_commands_parse() {
        for v in [serde_json::json!({ "type": "refresh" }), serde_json::json!({ "type": "polling", "value": 1000 })] {
            core::parse::<Cmd>(v).unwrap();
        }
    }
}
