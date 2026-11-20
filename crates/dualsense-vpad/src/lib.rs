//! User-mode channel to the Periph virtual DualSense kernel driver (`periph-vpad`, separate repository).
//!
//! The kernel device exposes `\\.\PeriphVpad`. Every IOCTL completes immediately: output
//! reports games write are queued in the driver (small FIFO) and user-mode drains the queue at
//! its own poll cadence. An empty queue completes successfully with 0 bytes.
//!
//! Closing the handle that started the virtual pad removes it (the driver tears it down in
//! EvtFileCleanup), so a crashed client never leaves a frozen controller behind.
//!
//! Tests use [`MockChannel`]; no kernel driver is needed to develop against this crate.

use anyhow::Result;
use dualsense_proto::feature::FeatureResponse;
use dualsense_proto::input::InputReport;
use dualsense_proto::output::OutputReport;

pub const DEVICE_PATH: &str = r"\\.\PeriphVpad";

/// Custom IOCTL codes, shared with the `periph-vpad` kernel driver repository (`src/ioctl.rs`
/// there; keep both in sync). METHOD_BUFFERED, FILE_ANY_ACCESS.
pub const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    (device_type << 16) | (access << 14) | (function << 2) | method
}

pub const FILE_DEVICE_UNKNOWN: u32 = 0x22;
pub const METHOD_BUFFERED: u32 = 0;
pub const FILE_ANY_ACCESS: u32 = 0;

pub const IOCTL_VPAD_SUBMIT_INPUT: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_ANY_ACCESS);
pub const IOCTL_VPAD_READ_OUTPUT: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 0x801, METHOD_BUFFERED, FILE_ANY_ACCESS);
pub const IOCTL_VPAD_SET_FEATURE: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 0x802, METHOD_BUFFERED, FILE_ANY_ACCESS);
pub const IOCTL_VPAD_GET_STATUS: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 0x803, METHOD_BUFFERED, FILE_ANY_ACCESS);
/// Create and start the virtual HID device (idempotent).
pub const IOCTL_VPAD_START: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 0x804, METHOD_BUFFERED, FILE_ANY_ACCESS);
/// Tear down the virtual HID device (emulation off / session closing).
pub const IOCTL_VPAD_DESTROY: u32 = ctl_code(FILE_DEVICE_UNKNOWN, 0x805, METHOD_BUFFERED, FILE_ANY_ACCESS);

/// Transport abstraction so the driver and tests can run without the kernel component.
pub trait Channel: Send {
    fn submit_input(&mut self, report: &InputReport) -> Result<()>;
    /// Returns the oldest queued output report, or `None` when the queue is empty.
    /// Never blocks. `Err` means the channel itself failed (driver gone, device removed).
    fn read_output(&mut self) -> Result<Option<OutputReport>>;
    fn set_feature_response(&mut self, response: &FeatureResponse) -> Result<()>;
    /// Whether the virtual HID device currently exists and is started.
    fn is_active(&mut self) -> Result<bool>;
    /// Creates and starts the virtual HID device (idempotent).
    fn start(&mut self) -> Result<()>;
    /// Removes the virtual HID device from the system (idempotent).
    fn destroy(&mut self) -> Result<()>;
}

#[cfg(windows)]
pub mod windows_channel {
    use super::*;
    use anyhow::{anyhow, Context};
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use std::ffi::c_void;

    pub struct WindowsChannel {
        handle: *mut c_void,
    }

    // The HANDLE is a plain OS handle usable from any thread; reads are serialized by the caller.
    unsafe impl Send for WindowsChannel {}

    impl WindowsChannel {
        pub fn open() -> Result<Self> {
            let path: Vec<u16> = DEVICE_PATH.encode_utf16().chain(std::iter::once(0)).collect();
            let handle = unsafe {
                CreateFileW(
                    path.as_ptr(),
                    FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                return Err(anyhow!(std::io::Error::last_os_error())
                    .context(format!("open {DEVICE_PATH} (virtual pad driver not installed?)")));
            }
            Ok(Self { handle })
        }

        fn ioctl(&mut self, code: u32, input: Option<&[u8]>, output: Option<&mut [u8]>) -> Result<u32> {
            let mut returned = 0u32;
            let (out_ptr, out_len) = match output {
                Some(buf) => (buf.as_mut_ptr() as *mut _, buf.len() as u32),
                None => (std::ptr::null_mut(), 0),
            };
            let (in_ptr, in_len) = match input {
                Some(buf) => (buf.as_ptr() as *const _, buf.len() as u32),
                None => (std::ptr::null(), 0),
            };
            let ok = unsafe {
                DeviceIoControl(
                    self.handle,
                    code,
                    in_ptr as *mut _,
                    in_len,
                    out_ptr,
                    out_len,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                return Err(std::io::Error::last_os_error()).context("DeviceIoControl");
            }
            Ok(returned)
        }
    }

    impl Channel for WindowsChannel {
        fn submit_input(&mut self, report: &InputReport) -> Result<()> {
            self.ioctl(IOCTL_VPAD_SUBMIT_INPUT, Some(report.as_bytes()), None)?;
            Ok(())
        }

        fn read_output(&mut self) -> Result<Option<OutputReport>> {
            let mut buf = [0u8; dualsense_proto::OUTPUT_REPORT_LEN];
            // 0 bytes = empty queue (the normal case); 48 = one report; anything else is a bug.
            match self.ioctl(IOCTL_VPAD_READ_OUTPUT, None, Some(&mut buf))? {
                0 => Ok(None),
                n if n as usize == buf.len() => Ok(Some(OutputReport::parse(&buf).map_err(anyhow::Error::msg)?)),
                n => Err(anyhow!("virtual pad returned a {n}-byte output report")),
            }
        }

        fn set_feature_response(&mut self, response: &FeatureResponse) -> Result<()> {
            self.ioctl(IOCTL_VPAD_SET_FEATURE, Some(&response.bytes), None)?;
            Ok(())
        }

        fn is_active(&mut self) -> Result<bool> {
            let mut buf = [0u8; 4];
            self.ioctl(IOCTL_VPAD_GET_STATUS, None, Some(&mut buf))?;
            Ok(u32::from_le_bytes(buf) & 1 != 0)
        }

        fn start(&mut self) -> Result<()> {
            self.ioctl(IOCTL_VPAD_START, None, None)?;
            Ok(())
        }

        fn destroy(&mut self) -> Result<()> {
            self.ioctl(IOCTL_VPAD_DESTROY, None, None)?;
            Ok(())
        }
    }

    impl Drop for WindowsChannel {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.handle) };
        }
    }
}

/// In-memory channel for unit tests and UI development without the kernel driver.
#[derive(Default)]
pub struct MockChannel {
    pub submitted: Vec<InputReport>,
    pub outputs: std::collections::VecDeque<OutputReport>,
    pub features: Vec<FeatureResponse>,
    pub active: bool,
}

impl Channel for MockChannel {
    fn submit_input(&mut self, report: &InputReport) -> Result<()> {
        self.submitted.push(*report);
        Ok(())
    }

    fn read_output(&mut self) -> Result<Option<OutputReport>> {
        Ok(self.outputs.pop_front())
    }

    fn set_feature_response(&mut self, response: &FeatureResponse) -> Result<()> {
        self.features.push(response.clone());
        Ok(())
    }

    fn is_active(&mut self) -> Result<bool> {
        Ok(self.active)
    }

    fn start(&mut self) -> Result<()> {
        self.active = true;
        Ok(())
    }

    fn destroy(&mut self) -> Result<()> {
        self.active = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ioctl_codes_are_stable() {
        // FILE_DEVICE_UNKNOWN(0x22)<<16 | function<<2 | METHOD_BUFFERED
        assert_eq!(IOCTL_VPAD_SUBMIT_INPUT, 0x222000);
        assert_eq!(IOCTL_VPAD_READ_OUTPUT, 0x222004);
        assert_eq!(IOCTL_VPAD_SET_FEATURE, 0x222008);
        assert_eq!(IOCTL_VPAD_GET_STATUS, 0x22200C);
        assert_eq!(IOCTL_VPAD_START, 0x222010);
        assert_eq!(IOCTL_VPAD_DESTROY, 0x222014);
    }

    #[test]
    fn mock_channel_records_traffic() {
        let mut ch = MockChannel::default();
        ch.active = true;
        let report = InputReport::neutral();
        ch.submit_input(&report).unwrap();
        assert!(ch.is_active().unwrap());
        assert_eq!(ch.submitted.len(), 1);

        let mut raw = [0u8; 48];
        raw[0] = 0x02;
        raw[3] = 99;
        ch.outputs.push_back(OutputReport::parse(&raw).unwrap());
        assert_eq!(ch.read_output().unwrap().unwrap().rumble(), (99, 0));
        assert!(ch.read_output().unwrap().is_none());
    }
}
