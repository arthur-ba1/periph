//! hidapi transport for the Rapoo VT2 Max.

use anyhow::{anyhow, bail, Context, Result};
use hidapi::{HidApi, HidDevice};
use std::time::{Duration, Instant};
use vt2_proto::{self as p, addr, ButtonAction, RespStatus, Status};

/// Error text when the dongle answers but the mouse doesn't (UI matches on "dormindo").
pub const MOUSE_ASLEEP: &str = "o mouse não respondeu (dormindo ou fora de alcance)";

pub struct Vt2 {
    cmd: HidDevice,
    status: Option<HidDevice>,
    pub product: String,
    pub serial: String,
    /// true = USB cable (PID 0x4623), false = 2.4 GHz dongle (PID 0x1423)
    pub wired: bool,
}

impl Vt2 {
    /// Open the mouse, preferring the cable when both cable and dongle are present.
    pub fn open() -> Result<Self> {
        let api = HidApi::new()?;
        for pid in [p::PID_WIRED, p::PID] {
            let mut cmd = None;
            let mut status = None;
            let (mut product, mut serial) = (String::new(), String::new());
            for d in api.device_list() {
                if d.vendor_id() != p::VID || d.product_id() != pid || d.usage_page() != p::CMD_USAGE_PAGE {
                    continue;
                }
                if d.usage() == p::CMD_USAGE {
                    cmd = Some(d.open_device(&api).context("open command interface")?);
                    product = d.product_string().unwrap_or_default().to_string();
                    serial = d.serial_number().unwrap_or_default().to_string();
                } else if d.usage() == p::STATUS_USAGE {
                    status = d.open_device(&api).ok();
                }
            }
            if let Some(cmd) = cmd {
                return Ok(Self { cmd, status, product, serial, wired: pid == p::PID_WIRED });
            }
        }
        Err(anyhow!("Rapoo VT2 Max not found (24AE:1423 dongle / 24AE:4623 cable)"))
    }

    fn transact(&self, cmd: Vec<u8>) -> Result<Vec<u8>> {
        self.cmd.write(&p::pad(cmd)).context("hid write")?;
        let deadline = Instant::now() + Duration::from_millis(1000);
        loop {
            let mut buf = [0u8; p::CMD_REPORT_LEN];
            buf[0] = p::CMD_REPORT_ID;
            // hidapi on Windows can report len+1 here; never trust it beyond the buffer.
            let n = self.cmd.get_input_report(&mut buf).context("get input report")?.min(buf.len());
            let (st, data) = p::parse_response(&buf[..n])?;
            match st {
                RespStatus::Ready => return Ok(data.to_vec()),
                RespStatus::Busy | RespStatus::Pending if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                RespStatus::Busy => bail!("device busy (timeout)"),
                RespStatus::Pending => bail!("{}", MOUSE_ASLEEP),
                RespStatus::Other(x) => bail!("unexpected response status 0x{x:02X}: {:02X?}", &buf[..n]),
            }
        }
    }

    pub fn read(&self, address: u16, len: usize) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(len);
        let mut a = address;
        while out.len() < len {
            let chunk = (len - out.len()).min(p::MAX_CHUNK);
            let data = self.transact(p::read_cmd(a, chunk as u8)?)?;
            if data.len() < chunk {
                bail!("short read at 0x{a:04X}");
            }
            out.extend_from_slice(&data[..chunk]);
            a += chunk as u16;
        }
        Ok(out)
    }

    pub fn write(&self, address: u16, data: &[u8]) -> Result<()> {
        for (i, chunk) in data.chunks(p::MAX_CHUNK).enumerate() {
            self.transact(p::write_cmd(address + (i * p::MAX_CHUNK) as u16, chunk)?)?;
        }
        Ok(())
    }

    /// Read-modify-write a single byte inside its 4-byte block, like the official app.
    pub fn set_byte(&self, address: u16, value: u8) -> Result<()> {
        let mut block = self.read(address, 4)?;
        block[0] = value;
        self.write(address, &block)
    }

    pub fn get_byte(&self, address: u16) -> Result<u8> {
        Ok(self.read(address, 1)?[0])
    }

    /// Wait for the next status report (sent every ~3 s).
    pub fn status(&self, timeout: Duration) -> Result<Option<Status>> {
        let Some(dev) = &self.status else { return Ok(None) };
        let end = Instant::now() + timeout;
        let mut buf = [0u8; 64];
        while Instant::now() < end {
            let n = dev.read_timeout(&mut buf, 200)?;
            if n > 0 {
                if let Ok(s) = p::parse_status(&buf[..n]) {
                    return Ok(Some(s));
                }
            }
        }
        Ok(None)
    }

    pub fn polling(&self) -> Result<u32> {
        Ok(p::polling_from_code(self.get_byte(addr::POLLING)?)?)
    }
    pub fn set_polling(&self, hz: u32) -> Result<()> {
        self.set_byte(addr::POLLING, p::polling_code(hz)?)
    }

    pub fn dpi_stages(&self) -> Result<(Vec<u16>, Vec<u16>, u8, u8)> {
        let x = self.read(addr::DPI_X, 0x14)?;
        let y = self.read(addr::DPI_Y, 14)?;
        let xs = x[..14].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let ys = y.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Ok((xs, ys, x[14], x[16])) // last index, active stage
    }

    /// Set the X and Y DPI of one stage (0-based).
    pub fn set_stage_dpi(&self, stage: usize, dpi: u16) -> Result<()> {
        if stage >= addr::DPI_STAGES {
            bail!("stage must be 0..{}", addr::DPI_STAGES);
        }
        let dpi = p::check_dpi(dpi)?.to_le_bytes();
        let off = 2 * stage;
        // Same shape as the official app: whole X block (0x14 bytes) then whole Y block (0x10).
        let mut x = self.read(addr::DPI_X, 0x14)?;
        let mut y = self.read(addr::DPI_Y, 0x10)?;
        x[off..off + 2].copy_from_slice(&dpi);
        y[off..off + 2].copy_from_slice(&dpi);
        self.write(addr::DPI_X, &x)?;
        self.write(addr::DPI_Y, &y)
    }

    pub fn set_active_stage(&self, stage: u8) -> Result<()> {
        let mut block = self.read(addr::DPI_ACTIVE, 4)?;
        block[0] = stage;
        block[2] = stage;
        self.write(addr::DPI_ACTIVE, &block)
    }

    pub fn button(&self, index: usize) -> Result<ButtonAction> {
        let b = self.read(p::button_addr(index), 4)?;
        Ok(ButtonAction::decode([b[0], b[1], b[2], b[3]]))
    }
    pub fn set_button(&self, index: usize, action: ButtonAction) -> Result<()> {
        if index >= addr::BUTTON_COUNT {
            bail!("button index must be 0..{}", addr::BUTTON_COUNT);
        }
        self.write(p::button_addr(index), &action.encode())
    }
}
