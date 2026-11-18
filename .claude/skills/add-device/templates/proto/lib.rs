//! Pure encoding/decoding for the <Brand Model> configuration protocol. No I/O.
//! Every fact here comes from docs/mydevice-protocol.md, which cites the capture it came from.
//!
//! TEMPLATE: the framing below (report 0x06, `A5 <op> <len> <addr_lo> <addr_hi>`, busy/ready byte)
//! is the VT2 Max one, kept as a worked example. Replace it with what your captures show.

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ProtoError {
    #[error("valor fora do intervalo permitido: {0}")]
    OutOfRange(String),
    #[error("resposta curta ou inválida")]
    BadResponse,
}

// ---- identity (from the probe) ----
pub const VID: u16 = 0x0000;
pub const PIDS: [u16; 1] = [0x0000]; // wired first, then dongle(s)
pub const CMD_USAGE_PAGE: u16 = 0xFF00;
pub const CMD_REPORT_ID: u8 = 0x06;
pub const CMD_REPORT_LEN: usize = 33;

// ---- framing ----
const OP_READ: u8 = 0xA4;
const OP_WRITE: u8 = 0xA5;

/// Memory map (addresses seen in the captures).
pub mod addr {
    pub const POLLING: u16 = 0x0880;
}

/// Build a read command, padded to the report length.
pub fn read_cmd(address: u16, len: u8) -> Vec<u8> {
    pad(vec![CMD_REPORT_ID, 0xA5, OP_READ, len, address as u8, (address >> 8) as u8, 0, 0])
}

/// Build a write command, padded to the report length.
pub fn write_cmd(address: u16, data: &[u8]) -> Vec<u8> {
    let mut v = vec![CMD_REPORT_ID, 0xA5, OP_WRITE, data.len() as u8, address as u8, (address >> 8) as u8, 0, 0];
    v.extend_from_slice(data);
    pad(v)
}

fn pad(mut v: Vec<u8>) -> Vec<u8> {
    v.resize(CMD_REPORT_LEN, 0);
    v
}

#[derive(Debug, PartialEq)]
pub enum Resp<'a> {
    Busy,
    Ready(&'a [u8]),
}

/// Parse an input report answer: byte 0 = 2 busy / 1 ready, data from byte 4.
pub fn parse_response(buf: &[u8]) -> Result<Resp<'_>, ProtoError> {
    match buf.first() {
        Some(2) => Ok(Resp::Busy),
        Some(1) if buf.len() >= 4 => Ok(Resp::Ready(&buf[4..])),
        _ => Err(ProtoError::BadResponse),
    }
}

// ---- value encodings: only values the official app offers ----
const POLLING: [(u32, u8); 4] = [(125, 0x08), (250, 0x04), (500, 0x02), (1000, 0x01)];

pub fn polling_code(hz: u32) -> Result<u8, ProtoError> {
    POLLING.iter().find(|p| p.0 == hz).map(|p| p.1).ok_or_else(|| ProtoError::OutOfRange(format!("{hz} Hz")))
}

pub fn polling_from_code(code: u8) -> Result<u32, ProtoError> {
    POLLING.iter().find(|p| p.1 == code).map(|p| p.0).ok_or(ProtoError::BadResponse)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hex string from the hidhook log -> bytes.
    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    /// Byte-for-byte equal to what the official app sent (marker "polling 1000").
    #[test]
    fn polling_write_matches_capture() {
        let sent = write_cmd(addr::POLLING, &[polling_code(1000).unwrap()]);
        assert_eq!(&sent[..9], hex("06 a5 a5 01 80 08 00 00 01").as_slice());
        assert_eq!(sent.len(), CMD_REPORT_LEN);
    }

    #[test]
    fn rejects_values_the_app_never_offers() {
        assert!(polling_code(333).is_err());
    }

    #[test]
    fn response_handshake() {
        assert_eq!(parse_response(&hex("02 00 00 00")), Ok(Resp::Busy));
        assert_eq!(parse_response(&hex("01 00 00 00 01")), Ok(Resp::Ready(&[1])));
    }
}
