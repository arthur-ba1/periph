//! Rapoo VT2 Max configuration protocol. See `docs/protocol.md`.
//!
//! Commands go out as output report 0x06:
//! `06 A5 <op> <len> <addr_lo> <addr_hi> 00 00 [data]` (op A4 = read, A5 = write).
//! Responses come back as input report 0x06: `<status> 00 00 00 [data]`.

use thiserror::Error;

pub const VID: u16 = 0x24AE;
/// 2.4 GHz dongle.
pub const PID: u16 = 0x1423;
/// Mouse plugged in over USB cable (same interfaces and report layout as the dongle).
pub const PID_WIRED: u16 = 0x4623;
pub const PIDS: [u16; 2] = [PID, PID_WIRED];
pub const CMD_USAGE_PAGE: u16 = 0xFF00;
pub const CMD_USAGE: u16 = 0x000E;
pub const STATUS_USAGE: u16 = 0x0002;
pub const CMD_REPORT_ID: u8 = 0x06;
pub const STATUS_REPORT_ID: u8 = 0x07;
/// Report length including the report id byte.
pub const CMD_REPORT_LEN: usize = 33;
/// Largest payload seen in a single read/write by the official app.
pub const MAX_CHUNK: usize = 20;

pub const OP_READ: u8 = 0xA4;
pub const OP_WRITE: u8 = 0xA5;

pub mod addr {
    pub const BUTTONS: u16 = 0x0600;
    pub const BUTTON_COUNT: usize = 10;
    pub const POLLING: u16 = 0x0880;
    pub const LOD: u16 = 0x0884;
    pub const MOTION_SYNC: u16 = 0x0885;
    pub const DPI_X: u16 = 0x0888;
    pub const DPI_LAST_INDEX: u16 = 0x0896;
    pub const DPI_ACTIVE: u16 = 0x0898;
    pub const DEBOUNCE_PRESS: u16 = 0x08C0;
    pub const DEBOUNCE_RELEASE: u16 = 0x08C1;
    pub const CORRECTION_FLAGS: u16 = 0x08C3;
    pub const SENSOR_ANGLE: u16 = 0x08C4;
    pub const GLASS: u16 = 0x08C5;
    pub const DPI_Y: u16 = 0x08C8;
    pub const DPI_STAGES: usize = 7;
}

#[derive(Debug, Error, PartialEq)]
pub enum ProtoError {
    #[error("payload too long ({0} > {MAX_CHUNK})")]
    TooLong(usize),
    #[error("unsupported polling rate {0} Hz (125/250/500/1000/2000/4000/8000)")]
    BadPolling(u32),
    #[error("unknown polling code 0x{0:02X}")]
    BadPollingCode(u8),
    #[error("DPI {0} out of range (50..=26000, step 10)")]
    BadDpi(u16),
    #[error("lift-off {0} mm out of range (0.7..=1.7)")]
    BadLod(f32),
    #[error("debounce {0} ms out of range (0..=32)")]
    BadDebounce(u8),
    #[error("response too short")]
    Short,
    #[error("not a status report")]
    NotStatus,
}

/// Build a read command (without trailing zero padding; callers pad to CMD_REPORT_LEN).
pub fn read_cmd(addr: u16, len: u8) -> Result<Vec<u8>, ProtoError> {
    if len as usize > 0x18 {
        return Err(ProtoError::TooLong(len as usize));
    }
    let [lo, hi] = addr.to_le_bytes();
    Ok(vec![CMD_REPORT_ID, OP_WRITE, OP_READ, len, lo, hi, 0, 0])
}

/// Build a write command.
pub fn write_cmd(addr: u16, data: &[u8]) -> Result<Vec<u8>, ProtoError> {
    if data.len() > MAX_CHUNK {
        return Err(ProtoError::TooLong(data.len()));
    }
    let [lo, hi] = addr.to_le_bytes();
    let mut v = vec![CMD_REPORT_ID, OP_WRITE, OP_WRITE, data.len() as u8, lo, hi, 0, 0];
    v.extend_from_slice(data);
    Ok(v)
}

/// Pad a command to the full output report length.
pub fn pad(mut cmd: Vec<u8>) -> Vec<u8> {
    cmd.resize(CMD_REPORT_LEN, 0);
    cmd
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespStatus {
    Ready,
    Busy,
    /// 0x00: the dongle echoed the request but the mouse hasn't answered (asleep / out of range).
    Pending,
    Other(u8),
}

/// Parse a command response (input report 0x06). Accepts buffers with or
/// without the leading report id. Returns status and the data bytes after the
/// 4-byte header.
pub fn parse_response(buf: &[u8]) -> Result<(RespStatus, &[u8]), ProtoError> {
    let b = if buf.first() == Some(&CMD_REPORT_ID) { &buf[1..] } else { buf };
    if b.len() < 4 {
        return Err(ProtoError::Short);
    }
    let st = match b[0] {
        0 => RespStatus::Pending,
        1 => RespStatus::Ready,
        2 => RespStatus::Busy,
        x => RespStatus::Other(x),
    };
    Ok((st, &b[4..]))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Status {
    pub dpi_stage: u8,
    pub dpi_x: u16,
    pub dpi_y: u16,
    pub battery: u8,
    pub polling_hz: Option<u32>,
    /// byte 1 bit 1: connected over USB cable (0x22) vs wireless (0x20)
    pub wired: bool,
    /// byte 7: 0x02 = charging, 0x01 = on battery
    pub charging: bool,
}

/// Parse the spontaneous status report 0x07 (19 bytes incl. id).
pub fn parse_status(buf: &[u8]) -> Result<Status, ProtoError> {
    if buf.len() < 12 {
        return Err(ProtoError::Short);
    }
    if buf[0] != STATUS_REPORT_ID || buf[1] & 0xF0 != 0x20 {
        return Err(ProtoError::NotStatus);
    }
    Ok(Status {
        dpi_stage: buf[2],
        dpi_x: u16::from_le_bytes([buf[3], buf[4]]),
        dpi_y: u16::from_le_bytes([buf[5], buf[6]]),
        battery: buf[8],
        polling_hz: polling_from_code(buf[10]).ok(),
        wired: buf[1] & 0x02 != 0,
        charging: buf[7] == 0x02,
    })
}

const POLLING: [(u32, u8); 7] =
    [(125, 0x08), (250, 0x04), (500, 0x02), (1000, 0x01), (2000, 0x84), (4000, 0x82), (8000, 0x81)];

pub fn polling_code(hz: u32) -> Result<u8, ProtoError> {
    POLLING.iter().find(|(h, _)| *h == hz).map(|(_, c)| *c).ok_or(ProtoError::BadPolling(hz))
}

pub fn polling_from_code(code: u8) -> Result<u32, ProtoError> {
    POLLING.iter().find(|(_, c)| *c == code).map(|(h, _)| *h).ok_or(ProtoError::BadPollingCode(code))
}

pub fn check_dpi(dpi: u16) -> Result<u16, ProtoError> {
    if (50..=26000).contains(&dpi) && dpi % 10 == 0 {
        Ok(dpi)
    } else {
        Err(ProtoError::BadDpi(dpi))
    }
}

/// Lift-off distance: raw = (mm - 0.6) / 0.1, UI range 0.7..=1.7 mm.
pub fn lod_raw(mm: f32) -> Result<u8, ProtoError> {
    if !(0.7..=1.7001).contains(&mm) {
        return Err(ProtoError::BadLod(mm));
    }
    Ok(((mm - 0.6) * 10.0).round() as u8)
}

pub fn lod_mm(raw: u8) -> f32 {
    0.6 + raw as f32 * 0.1
}

pub fn check_debounce(ms: u8) -> Result<u8, ProtoError> {
    if ms <= 32 { Ok(ms) } else { Err(ProtoError::BadDebounce(ms)) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ButtonAction {
    /// Mouse button bitmask: 1 left, 2 right, 4 middle, 8 back, 0x10 forward.
    Mouse(u8),
    /// Keyboard key by HID usage id (0x04 = A).
    Key(u8),
    DpiUp,
    DpiDown,
    /// Wheel function, raw code (not fully decoded).
    Wheel(u8),
    Disabled,
    Raw([u8; 4]),
}

impl ButtonAction {
    pub fn encode(self) -> [u8; 4] {
        match self {
            ButtonAction::Mouse(m) => [0x03, 0, m, 0],
            ButtonAction::Key(k) => [0x00, 0, k, 0],
            ButtonAction::DpiUp => [0x08, 0, 5, 0],
            ButtonAction::DpiDown => [0x08, 0, 6, 0],
            ButtonAction::Wheel(c) => [0x07, 0, c, 0],
            ButtonAction::Disabled => [0xFF; 4],
            ButtonAction::Raw(r) => r,
        }
    }

    pub fn decode(b: [u8; 4]) -> Self {
        match b {
            [0x03, 0, m, 0] => ButtonAction::Mouse(m),
            [0x00, 0, k, 0] => ButtonAction::Key(k),
            [0x08, 0, 5, 0] => ButtonAction::DpiUp,
            [0x08, 0, 6, 0] => ButtonAction::DpiDown,
            [0x07, 0, c, 0] => ButtonAction::Wheel(c),
            [0xFF, 0xFF, 0xFF, 0xFF] => ButtonAction::Disabled,
            r => ButtonAction::Raw(r),
        }
    }
}

pub const BUTTON_NAMES: [&str; addr::BUTTON_COUNT] =
    ["left", "middle", "right", "dpi+", "dpi-", "forward", "back", "wheel-up", "wheel-down", "diy"];

pub fn button_addr(index: usize) -> u16 {
    addr::BUTTONS + 4 * index as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    // Fixtures copied from a hidhook capture of A HUB traffic (raw log not published).
    #[test]
    fn read_cmd_matches_capture() {
        assert_eq!(read_cmd(0x0898, 4).unwrap(), hex("06 A5 A4 04 98 08 00 00"));
        assert_eq!(read_cmd(0x0600, 0x14).unwrap(), hex("06 A5 A4 14 00 06 00 00"));
    }

    #[test]
    fn write_cmds_match_capture() {
        assert_eq!(
            write_cmd(addr::POLLING, &[polling_code(1000).unwrap(), 0x06, 0xFF, 0xFF]).unwrap(),
            hex("06 A5 A5 04 80 08 00 00 01 06 FF FF")
        );
        assert_eq!(
            write_cmd(button_addr(6), &ButtonAction::Mouse(4).encode()).unwrap(),
            hex("06 A5 A5 04 18 06 00 00 03 00 04 00")
        );
        assert_eq!(
            write_cmd(button_addr(6), &ButtonAction::Key(0x04).encode()).unwrap(),
            hex("06 A5 A5 04 18 06 00 00 00 00 04 00")
        );
        let dpis: Vec<u8> = [400u16, 810, 1200, 1600, 3200, 6400, 26000]
            .iter()
            .flat_map(|d| d.to_le_bytes())
            .chain([0x06, 0x00, 0x01, 0x00, 0x01, 0x02])
            .collect();
        assert_eq!(
            write_cmd(addr::DPI_X, &dpis).unwrap(),
            hex("06 A5 A5 14 88 08 00 00 90 01 2A 03 B0 04 40 06 80 0C 00 19 90 65 06 00 01 00 01 02")
        );
    }

    #[test]
    fn polling_table() {
        for (hz, code) in POLLING {
            assert_eq!(polling_code(hz).unwrap(), code);
            assert_eq!(polling_from_code(code).unwrap(), hz);
        }
        assert!(polling_code(300).is_err());
    }

    #[test]
    fn status_report() {
        let s = parse_status(&hex("07 20 01 2A 03 2A 03 01 53 00 84 84 00 00 00 00 06 04 00")).unwrap();
        assert_eq!(s, Status { dpi_stage: 1, dpi_x: 810, dpi_y: 810, battery: 83, polling_hz: Some(2000), wired: false, charging: false });
        // captured over USB cable while charging
        let w = parse_status(&hex("07 22 02 B0 04 B0 04 02 55 00 01 01 00 00 00 00 20 00 00")).unwrap();
        assert_eq!(w, Status { dpi_stage: 2, dpi_x: 1200, dpi_y: 1200, battery: 85, polling_hz: Some(1000), wired: true, charging: true });
    }

    #[test]
    fn response_with_and_without_id() {
        let r = hex("01 00 00 00 02 00 02 01");
        assert_eq!(parse_response(&r).unwrap(), (RespStatus::Ready, &[2u8, 0, 2, 1][..]));
        let mut with_id = vec![6];
        with_id.extend(&r);
        assert_eq!(parse_response(&with_id).unwrap().0, RespStatus::Ready);
        assert_eq!(parse_response(&hex("02 00 00 00 00")).unwrap().0, RespStatus::Busy);
    }

    #[test]
    fn lod_conversion() {
        assert_eq!(lod_raw(1.1).unwrap(), 5);
        assert_eq!(lod_raw(1.2).unwrap(), 6);
        assert!((lod_mm(5) - 1.1).abs() < 1e-4);
        assert!(lod_raw(2.0).is_err());
    }

    #[test]
    fn button_roundtrip() {
        for a in [ButtonAction::Mouse(1), ButtonAction::Key(4), ButtonAction::DpiUp, ButtonAction::Disabled] {
            assert_eq!(ButtonAction::decode(a.encode()), a);
        }
    }
}
