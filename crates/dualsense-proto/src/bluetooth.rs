//! Bluetooth framing. Over Bluetooth the controller carries the same data as over USB inside
//! other report ids, with a CRC-32 (zlib polynomial) appended. The CRC covers a one-byte seed
//! followed by the report minus its last four bytes, and is stored little-endian in those bytes.
//!
//! Observed from the official software with the user's controller paired
//! (`captures/dsx-bt.jsonl`, see `docs/dualsense-protocol.md`):
//! - output 0x31, 78 bytes: `31 02`, the 47-byte USB output body (USB bytes 1..48), zeros,
//!   CRC (seed 0xA2). Byte 1 was 0x02 in every captured report;
//! - feature reports: same ids and lengths as USB, last four bytes are a CRC (seed 0xA3)
//!   where USB has zeros.
//!
//! Input 0x31 (78 bytes, seed 0xA1) carries the USB input body (USB bytes 1..64) from byte 2,
//! as in Linux `hid-playstation`. The controller sends it once its calibration feature report
//! (0x05) has been read; before that it sends a reduced 0x01 report, which is ignored here.

use crate::input::InputReport;
use crate::output::OutputReport;
use crate::{INPUT_REPORT_ID, INPUT_REPORT_LEN, OUTPUT_REPORT_LEN};

pub const INPUT_REPORT_ID_BT: u8 = 0x31;
pub const INPUT_REPORT_LEN_BT: usize = 78;
pub const OUTPUT_REPORT_ID_BT: u8 = 0x31;
pub const OUTPUT_REPORT_LEN_BT: usize = 78;
/// Byte 1 of every output report the official software sent.
pub const OUTPUT_TAG: u8 = 0x02;

const INPUT_SEED: u8 = 0xA1;
const OUTPUT_SEED: u8 = 0xA2;
const FEATURE_SEED: u8 = 0xA3;

/// USB input body (after the id) starts here in the Bluetooth input report.
const INPUT_BODY: usize = 2;
/// USB output body (after the id) starts here in the Bluetooth output report.
const OUTPUT_BODY: usize = 2;

/// CRC-32 (reflected, polynomial 0xEDB88320, as zlib) of `seed` followed by `data`.
pub fn crc32(seed: u8, data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in std::iter::once(&seed).chain(data) {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn crc_ok(seed: u8, report: &[u8]) -> bool {
    let n = report.len();
    n > 4 && crc32(seed, &report[..n - 4]).to_le_bytes() == report[n - 4..]
}

/// Bluetooth input 0x31 → the equivalent USB input report. Accepts a longer buffer (Windows
/// pads reads to the descriptor's largest input report) and checks only the first 78 bytes.
pub fn parse_input(bytes: &[u8]) -> Result<InputReport, &'static str> {
    if bytes.len() < INPUT_REPORT_LEN_BT {
        return Err("bluetooth input report too short");
    }
    let report = &bytes[..INPUT_REPORT_LEN_BT];
    if report[0] != INPUT_REPORT_ID_BT {
        return Err("not a full bluetooth input report");
    }
    if !crc_ok(INPUT_SEED, report) {
        return Err("bluetooth input report crc mismatch");
    }
    let mut usb = [0u8; INPUT_REPORT_LEN];
    usb[0] = INPUT_REPORT_ID;
    usb[1..].copy_from_slice(&report[INPUT_BODY..INPUT_BODY + INPUT_REPORT_LEN - 1]);
    InputReport::parse(&usb)
}

/// USB output report 0x02 → Bluetooth output report 0x31 with the observed framing.
pub fn output_report(usb: &OutputReport) -> [u8; OUTPUT_REPORT_LEN_BT] {
    let mut bt = [0u8; OUTPUT_REPORT_LEN_BT];
    bt[0] = OUTPUT_REPORT_ID_BT;
    bt[1] = OUTPUT_TAG;
    bt[OUTPUT_BODY..OUTPUT_BODY + OUTPUT_REPORT_LEN - 1].copy_from_slice(&usb.as_bytes()[1..]);
    let crc = crc32(OUTPUT_SEED, &bt[..OUTPUT_REPORT_LEN_BT - 4]);
    bt[OUTPUT_REPORT_LEN_BT - 4..].copy_from_slice(&crc.to_le_bytes());
    bt
}

/// A feature report read over Bluetooth (`len` bytes, CRC at the end) → its USB form, with the
/// CRC bytes zeroed as the USB controller reports them.
pub fn feature_to_usb(bytes: &[u8], len: usize) -> Result<Vec<u8>, &'static str> {
    if bytes.len() < len {
        return Err("bluetooth feature report too short");
    }
    let report = &bytes[..len];
    if !crc_ok(FEATURE_SEED, report) {
        return Err("bluetooth feature report crc mismatch");
    }
    let mut usb = report.to_vec();
    usb[len - 4..].fill(0);
    Ok(usb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    /// First 0x31 report the official software sent after connecting (bytes 0..49; the rest
    /// up to the CRC is zero).
    const CAPTURED_OUTPUT_HEAD: &str = "31 02 0f 55 00 00 00 00 00 00 00 0f 25 ff ff ff ff ff ff ff 00 00 00 22 ff ff ff ff ff ff ff 00 00 00 00 00 00 00 00 00 07 00 00 01 00 05 00 00 00";
    const CAPTURED_OUTPUT_CRC: [u8; 4] = [0x29, 0xc3, 0xd8, 0xdb];

    #[test]
    fn crc_matches_captured_output() {
        let mut bt = [0u8; OUTPUT_REPORT_LEN_BT];
        let head = hex(CAPTURED_OUTPUT_HEAD);
        bt[..head.len()].copy_from_slice(&head);
        assert_eq!(crc32(OUTPUT_SEED, &bt[..74]).to_le_bytes(), CAPTURED_OUTPUT_CRC);
    }

    #[test]
    fn usb_output_converts_to_the_captured_bluetooth_report() {
        let head = hex(CAPTURED_OUTPUT_HEAD);
        let mut usb = [0u8; OUTPUT_REPORT_LEN];
        usb[0] = 0x02;
        usb[1..].copy_from_slice(&head[2..49]);
        let bt = output_report(&OutputReport::parse(&usb).unwrap());
        assert_eq!(&bt[..49], &head[..]);
        assert!(bt[49..74].iter().all(|&b| b == 0));
        assert_eq!(bt[74..], CAPTURED_OUTPUT_CRC);
    }

    #[test]
    fn captured_calibration_feature_converts_to_usb_form() {
        // Feature 0x05 as read over Bluetooth by the official software.
        let bt = hex("05 fe ff fe ff fa ff 8b 22 77 dd 9c 22 60 dd 79 22 6e dd 1c 02 1c 02 fb 1f ea df b5 1f a4 df f3 1f e8 df 07 00 1d 92 f7 2f");
        let usb = feature_to_usb(&bt, 41).unwrap();
        assert_eq!(&usb[..37], &bt[..37]);
        assert_eq!(&usb[37..], &[0, 0, 0, 0]);
        let mut bad = bt.clone();
        bad[3] ^= 1;
        assert!(feature_to_usb(&bad, 41).is_err());
    }

    #[test]
    fn input_round_trip_through_bluetooth_framing() {
        let mut usb = InputReport::neutral();
        usb.set_sticks(1, 2, 3, 4);
        let mut bt = vec![0u8; 547]; // Windows pads reads to the largest input report
        bt[0] = INPUT_REPORT_ID_BT;
        bt[INPUT_BODY..INPUT_BODY + 63].copy_from_slice(&usb.as_bytes()[1..]);
        let crc = crc32(INPUT_SEED, &bt[..74]);
        bt[74..78].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(parse_input(&bt).unwrap(), usb);
        bt[10] ^= 1;
        assert!(parse_input(&bt).is_err(), "crc must be checked");
        assert!(parse_input(&[0x01; 10]).is_err(), "reduced report is ignored");
    }
}
