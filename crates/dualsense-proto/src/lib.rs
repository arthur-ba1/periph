//! Sony DualSense (VID 054C / PID 0CE6) HID report encoding/decoding: USB reports, plus the
//! Bluetooth framing in [`bluetooth`].
//!
//! Pure crate: no I/O. Used by the Periph `dualsense` driver to read the physical controller
//! and by the virtual-pad channel to build the reports the virtual device presents to games.
//!
//! Sources (all read-only observations):
//! - the controller's own 437-byte USB HID report descriptor (`docs/dualsense-protocol.md`);
//! - the output/feature layout documented in `docs/dualsense-protocol.md`,
//!   cross-checked against Linux `hid-playstation`;
//! - GET_FEATURE reads of the user's own controller (`vt2-probe --vid 054c --ds-features`).
//!
//! Nothing here writes firmware, bootloader, pairing or calibration commands: feature reports
//! are only *answered* by the virtual device with data captured from the physical one.

pub mod bluetooth;
pub mod descriptor;
pub mod effects;
pub mod feature;
pub mod input;
pub mod output;

pub const VID: u16 = 0x054C;
pub const PID: u16 = 0x0CE6;

pub const INPUT_REPORT_ID: u8 = 0x01;
pub const INPUT_REPORT_LEN: usize = 64;
pub const OUTPUT_REPORT_ID: u8 = 0x02;
pub const OUTPUT_REPORT_LEN: usize = 48;
