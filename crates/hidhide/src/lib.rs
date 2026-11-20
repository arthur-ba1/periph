//! Client for the HidHide filter driver (<https://github.com/nefarius/HidHide>, MIT).
//!
//! HidHide is a signed upper filter on HID devices: devices in its *blacklist* cannot be opened
//! by any process, except the applications in its *whitelist*. Periph uses it while a virtual
//! controller is active, so games only see the virtual one and not the physical pad as well.
//!
//! Control interface (from the HidHide sources, `Shared/HidHideIoctlContract.h` and
//! `HidHideCLI/src/FilterDriverProxy.cpp`):
//! - control device `\\.\HidHide`, no elevation needed, **one open handle at a time**, so the
//!   handle is opened per operation and closed right after;
//! - lists are UTF-16 MULTI_SZ (each entry NUL-terminated, plus a final NUL); GET is called once
//!   with no buffer to learn the size in bytes, then with the buffer;
//! - the blacklist holds device instance ids (`HID\VID_054C&PID_0CE6&MI_03\...`), the whitelist
//!   full image paths in NT device form (`\Device\HarddiskVolume3\...\periph.exe`);
//! - ACTIVE and WLINVERSE are one BOOLEAN byte.
//!
//! The HidHide docs ask third-party apps to extend the user's lists, never replace them: this
//! crate only adds the entries it needs and removes exactly those afterwards. What was added is
//! also written to a ledger file, so a crash can be undone at the next start ([`recover`]).

use anyhow::{Context, Result};
use std::path::Path;

pub const DEVICE_PATH: &str = r"\\.\HidHide";

const DEVICE_TYPE: u32 = 32769;
const METHOD_BUFFERED: u32 = 0;
const FILE_READ_DATA: u32 = 1;

const fn ctl_code(function: u32) -> u32 {
    (DEVICE_TYPE << 16) | (FILE_READ_DATA << 14) | (function << 2) | METHOD_BUFFERED
}

pub const IOCTL_GET_WHITELIST: u32 = ctl_code(2048);
pub const IOCTL_SET_WHITELIST: u32 = ctl_code(2049);
pub const IOCTL_GET_BLACKLIST: u32 = ctl_code(2050);
pub const IOCTL_SET_BLACKLIST: u32 = ctl_code(2051);
pub const IOCTL_GET_ACTIVE: u32 = ctl_code(2052);
pub const IOCTL_SET_ACTIVE: u32 = ctl_code(2053);
pub const IOCTL_GET_WLINVERSE: u32 = ctl_code(2054);

/// Encodes a list as MULTI_SZ bytes (UTF-16LE). An empty list is a single NUL.
pub fn encode_multi_sz(list: &[String]) -> Vec<u8> {
    let mut wide: Vec<u16> = Vec::new();
    for s in list {
        wide.extend(s.encode_utf16());
        wide.push(0);
    }
    wide.push(0);
    wide.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// Decodes MULTI_SZ bytes (UTF-16LE), skipping empty runs like the official client does.
pub fn decode_multi_sz(bytes: &[u8]) -> Vec<String> {
    let wide: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    wide.split(|&w| w == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect()
}

/// Instance ids and paths are case-insensitive on Windows.
fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Adds `entry` unless an equivalent one is present. Returns whether the list changed.
pub fn add_entry(list: &mut Vec<String>, entry: &str) -> bool {
    if list.iter().any(|e| same(e, entry)) {
        return false;
    }
    list.push(entry.to_string());
    true
}

/// Removes every entry equivalent to `entry`. Returns whether the list changed.
pub fn remove_entry(list: &mut Vec<String>, entry: &str) -> bool {
    let before = list.len();
    list.retain(|e| !same(e, entry));
    list.len() != before
}

/// Operations on the HidHide configuration. Implemented by the real control device and by
/// [`MockControl`] for tests.
pub trait Control {
    fn blacklist(&mut self) -> Result<Vec<String>>;
    fn set_blacklist(&mut self, list: &[String]) -> Result<()>;
    fn whitelist(&mut self) -> Result<Vec<String>>;
    fn set_whitelist(&mut self, list: &[String]) -> Result<()>;
    fn active(&mut self) -> Result<bool>;
    fn set_active(&mut self, on: bool) -> Result<()>;
    /// Inverse mode: whitelisted applications are the ones that *cannot* see hidden devices.
    fn inverse(&mut self) -> Result<bool>;
}

/// What Periph changed in the HidHide configuration, so it can be undone exactly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    /// Device instance id we added to the blacklist (None if the user already had it).
    pub device: Option<String>,
    /// Image path we added to the whitelist (None if already there, or inverse mode is on).
    pub app: Option<String>,
    /// Whether we turned hiding on (it was off before).
    pub activated: bool,
}

impl Applied {
    pub fn is_empty(&self) -> bool {
        self.device.is_none() && self.app.is_none() && !self.activated
    }

    /// Ledger file contents: one `kind<TAB>value` line per change.
    pub fn to_ledger(&self) -> String {
        let mut s = String::new();
        if let Some(d) = &self.device {
            s += &format!("device\t{d}\n");
        }
        if let Some(a) = &self.app {
            s += &format!("app\t{a}\n");
        }
        if self.activated {
            s += "active\t1\n";
        }
        s
    }

    pub fn from_ledger(text: &str) -> Self {
        let mut a = Applied::default();
        for line in text.lines() {
            match line.split_once('\t') {
                Some(("device", v)) if !v.is_empty() => a.device = Some(v.to_string()),
                Some(("app", v)) if !v.is_empty() => a.app = Some(v.to_string()),
                Some(("active", "1")) => a.activated = true,
                _ => {}
            }
        }
        a
    }
}

/// Hides `device` from everyone but `app`, extending the user's lists. The whitelist is
/// written before the blacklist so Periph never loses access to the pad in between.
pub fn hide_with(ctl: &mut dyn Control, device: &str, app: &str) -> Result<Applied> {
    let mut applied = Applied::default();
    if !ctl.inverse()? {
        let mut white = ctl.whitelist()?;
        if add_entry(&mut white, app) {
            ctl.set_whitelist(&white)?;
            applied.app = Some(app.to_string());
        }
    }
    let mut black = ctl.blacklist()?;
    if add_entry(&mut black, device) {
        ctl.set_blacklist(&black)?;
        applied.device = Some(device.to_string());
    }
    if !ctl.active()? {
        ctl.set_active(true)?;
        applied.activated = true;
    }
    Ok(applied)
}

/// Undoes exactly what [`hide_with`] changed; entries the user had before are left alone.
pub fn unhide_with(ctl: &mut dyn Control, applied: &Applied) -> Result<()> {
    if let Some(device) = &applied.device {
        let mut black = ctl.blacklist()?;
        if remove_entry(&mut black, device) {
            ctl.set_blacklist(&black)?;
        }
    }
    if let Some(app) = &applied.app {
        let mut white = ctl.whitelist()?;
        if remove_entry(&mut white, app) {
            ctl.set_whitelist(&white)?;
        }
    }
    if applied.activated {
        ctl.set_active(false)?;
    }
    Ok(())
}

/// Hides `device_instance_id` from every process except `app_image`. Returns `None` when
/// HidHide is not installed. The changes are recorded in `ledger` until [`unhide`].
pub fn hide(device_instance_id: &str, app_image: &Path, ledger: &Path) -> Result<Option<Applied>> {
    let app = nt_image_path(app_image)?;
    let Some(mut ctl) = open()? else {
        return Ok(None);
    };
    let applied = hide_with(&mut ctl, device_instance_id, &app)?;
    drop(ctl);
    if !applied.is_empty() {
        if let Some(dir) = ledger.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(ledger, applied.to_ledger()).context("write HidHide ledger")?;
    }
    Ok(Some(applied))
}

/// Reverts [`hide`] and removes the ledger.
pub fn unhide(applied: &Applied, ledger: &Path) -> Result<()> {
    if !applied.is_empty() {
        if let Some(mut ctl) = open()? {
            unhide_with(&mut ctl, applied)?;
        }
    }
    let _ = std::fs::remove_file(ledger);
    Ok(())
}

/// Reverts changes left behind by a previous run that ended without [`unhide`] (crash, power
/// loss). Cheap no-op when there is no ledger.
pub fn recover(ledger: &Path) -> Result<()> {
    let Ok(text) = std::fs::read_to_string(ledger) else {
        return Ok(());
    };
    unhide(&Applied::from_ledger(&text), ledger)
}

/// Whether the HidHide control device exists (driver installed and running).
pub fn installed() -> bool {
    matches!(open(), Ok(Some(_)))
}

/// `\Device\HarddiskVolume3` + `C:\` + `C:\Apps\periph.exe` -> `\Device\HarddiskVolume3\Apps\periph.exe`
pub fn join_nt_path(device: &str, mount: &str, full: &str) -> Option<String> {
    let rest = full.get(..mount.len()).filter(|p| same(p, mount)).map(|_| &full[mount.len()..])?;
    let rest = rest.trim_start_matches('\\');
    Some(format!("{}\\{}", device.trim_end_matches('\\'), rest))
}

#[cfg(windows)]
pub use sys::{nt_image_path, open, WindowsControl};

#[cfg(windows)]
mod sys {
    use super::*;
    use anyhow::{anyhow, bail};
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, GENERIC_READ, HANDLE,
        INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, GetVolumeNameForVolumeMountPointW, GetVolumePathNameW, QueryDosDeviceW,
        FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn from_wide(buf: &[u16]) -> String {
        let end = buf.iter().position(|&w| w == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    /// An open handle to `\\.\HidHide`. Keep it short-lived: HidHide allows one at a time.
    pub struct WindowsControl {
        handle: HANDLE,
    }

    /// Opens the control device. `Ok(None)` when HidHide is not installed. Retries briefly while
    /// another process (e.g. the HidHide configuration client) holds the exclusive handle.
    pub fn open() -> Result<Option<WindowsControl>> {
        let path = wide(DEVICE_PATH);
        let mut last = 0;
        for _ in 0..10 {
            let handle = unsafe {
                CreateFileW(
                    path.as_ptr(),
                    GENERIC_READ,
                    FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    std::ptr::null_mut(),
                )
            };
            if handle != INVALID_HANDLE_VALUE {
                return Ok(Some(WindowsControl { handle }));
            }
            last = unsafe { GetLastError() };
            if last == ERROR_FILE_NOT_FOUND || last == ERROR_PATH_NOT_FOUND {
                return Ok(None);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Err(std::io::Error::from_raw_os_error(last as i32)).context("open HidHide control device")
    }

    impl WindowsControl {
        fn ioctl(&mut self, code: u32, input: &[u8], output: &mut [u8]) -> Result<usize> {
            let mut returned = 0u32;
            let in_ptr = if input.is_empty() { std::ptr::null() } else { input.as_ptr() as *const c_void };
            let out_ptr = if output.is_empty() { std::ptr::null_mut() } else { output.as_mut_ptr() as *mut c_void };
            let ok = unsafe {
                DeviceIoControl(
                    self.handle,
                    code,
                    in_ptr,
                    input.len() as u32,
                    out_ptr,
                    output.len() as u32,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                return Err(std::io::Error::last_os_error()).with_context(|| format!("HidHide IOCTL {code:#x}"));
            }
            Ok(returned as usize)
        }

        fn get_list(&mut self, code: u32) -> Result<Vec<String>> {
            let needed = self.ioctl(code, &[], &mut [])?;
            if needed == 0 {
                return Ok(Vec::new());
            }
            let mut buf = vec![0u8; needed + (needed & 1)];
            let n = self.ioctl(code, &[], &mut buf)?;
            Ok(decode_multi_sz(&buf[..n.min(buf.len())]))
        }

        fn get_bool(&mut self, code: u32) -> Result<bool> {
            let mut b = [0u8; 1];
            let n = self.ioctl(code, &[], &mut b)?;
            if n != 1 {
                bail!("HidHide IOCTL {code:#x} returned {n} bytes");
            }
            Ok(b[0] != 0)
        }
    }

    impl Control for WindowsControl {
        fn blacklist(&mut self) -> Result<Vec<String>> {
            self.get_list(IOCTL_GET_BLACKLIST)
        }
        fn set_blacklist(&mut self, list: &[String]) -> Result<()> {
            self.ioctl(IOCTL_SET_BLACKLIST, &encode_multi_sz(list), &mut []).map(drop)
        }
        fn whitelist(&mut self) -> Result<Vec<String>> {
            self.get_list(IOCTL_GET_WHITELIST)
        }
        fn set_whitelist(&mut self, list: &[String]) -> Result<()> {
            self.ioctl(IOCTL_SET_WHITELIST, &encode_multi_sz(list), &mut []).map(drop)
        }
        fn active(&mut self) -> Result<bool> {
            self.get_bool(IOCTL_GET_ACTIVE)
        }
        fn set_active(&mut self, on: bool) -> Result<()> {
            self.ioctl(IOCTL_SET_ACTIVE, &[on as u8], &mut []).map(drop)
        }
        fn inverse(&mut self) -> Result<bool> {
            self.get_bool(IOCTL_GET_WLINVERSE)
        }
    }

    impl Drop for WindowsControl {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.handle) };
        }
    }

    /// `C:\Apps\periph.exe` -> `\Device\HarddiskVolume3\Apps\periph.exe` (the form HidHide's
    /// whitelist uses): volume mount point -> volume GUID name -> NT device name.
    pub fn nt_image_path(path: &Path) -> Result<String> {
        let full = path.to_string_lossy();
        let full = full.strip_prefix(r"\\?\").unwrap_or(&full).to_string();
        let mut mount = [0u16; 260];
        if unsafe { GetVolumePathNameW(wide(&full).as_ptr(), mount.as_mut_ptr(), mount.len() as u32) } == 0 {
            return Err(std::io::Error::last_os_error()).context("GetVolumePathNameW");
        }
        let mount = from_wide(&mount);
        let mut volume = [0u16; 64];
        if unsafe { GetVolumeNameForVolumeMountPointW(wide(&mount).as_ptr(), volume.as_mut_ptr(), volume.len() as u32) } == 0 {
            return Err(std::io::Error::last_os_error()).context("GetVolumeNameForVolumeMountPointW");
        }
        // `\\?\Volume{guid}\` -> `Volume{guid}`
        let volume = from_wide(&volume);
        let name = volume.trim_start_matches(r"\\?\").trim_end_matches('\\');
        let mut device = [0u16; 260];
        if unsafe { QueryDosDeviceW(wide(name).as_ptr(), device.as_mut_ptr(), device.len() as u32) } == 0 {
            return Err(std::io::Error::last_os_error()).context("QueryDosDeviceW");
        }
        join_nt_path(&from_wide(&device), &mount, &full).ok_or_else(|| anyhow!("{full} is not under {mount}"))
    }
}

#[cfg(not(windows))]
pub struct WindowsControl;

#[cfg(not(windows))]
impl Control for WindowsControl {
    fn blacklist(&mut self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    fn set_blacklist(&mut self, _: &[String]) -> Result<()> {
        Ok(())
    }
    fn whitelist(&mut self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    fn set_whitelist(&mut self, _: &[String]) -> Result<()> {
        Ok(())
    }
    fn active(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn set_active(&mut self, _: bool) -> Result<()> {
        Ok(())
    }
    fn inverse(&mut self) -> Result<bool> {
        Ok(false)
    }
}

#[cfg(not(windows))]
pub fn open() -> Result<Option<WindowsControl>> {
    Ok(None)
}

#[cfg(not(windows))]
pub fn nt_image_path(path: &Path) -> Result<String> {
    Ok(path.to_string_lossy().into_owned())
}

/// In-memory HidHide configuration for tests.
#[derive(Debug, Default, Clone)]
pub struct MockControl {
    pub black: Vec<String>,
    pub white: Vec<String>,
    pub active: bool,
    pub inverse: bool,
}

impl Control for MockControl {
    fn blacklist(&mut self) -> Result<Vec<String>> {
        Ok(self.black.clone())
    }
    fn set_blacklist(&mut self, list: &[String]) -> Result<()> {
        self.black = list.to_vec();
        Ok(())
    }
    fn whitelist(&mut self) -> Result<Vec<String>> {
        Ok(self.white.clone())
    }
    fn set_whitelist(&mut self, list: &[String]) -> Result<()> {
        self.white = list.to_vec();
        Ok(())
    }
    fn active(&mut self) -> Result<bool> {
        Ok(self.active)
    }
    fn set_active(&mut self, on: bool) -> Result<()> {
        self.active = on;
        Ok(())
    }
    fn inverse(&mut self) -> Result<bool> {
        Ok(self.inverse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAD: &str = r"HID\VID_054C&PID_0CE6&MI_03\9&1A5951D8&0&0003";
    const APP: &str = r"\Device\HarddiskVolume3\Periph\periph.exe";

    #[test]
    fn ioctl_codes_match_the_hidhide_contract() {
        assert_eq!(IOCTL_GET_WHITELIST, 0x8001_6000);
        assert_eq!(IOCTL_SET_WHITELIST, 0x8001_6004);
        assert_eq!(IOCTL_GET_BLACKLIST, 0x8001_6008);
        assert_eq!(IOCTL_SET_BLACKLIST, 0x8001_600C);
        assert_eq!(IOCTL_GET_ACTIVE, 0x8001_6010);
        assert_eq!(IOCTL_SET_ACTIVE, 0x8001_6014);
        assert_eq!(IOCTL_GET_WLINVERSE, 0x8001_6018);
    }

    #[test]
    fn multi_sz_round_trip() {
        assert_eq!(encode_multi_sz(&[]), vec![0, 0]);
        let list = vec!["AB".to_string(), "c".to_string()];
        let bytes = encode_multi_sz(&list);
        assert_eq!(bytes, vec![b'A', 0, b'B', 0, 0, 0, b'c', 0, 0, 0, 0, 0]);
        assert_eq!(decode_multi_sz(&bytes), list);
        assert!(decode_multi_sz(&[0, 0]).is_empty());
    }

    #[test]
    fn hide_extends_lists_and_unhide_restores_them() {
        let mut ctl = MockControl { black: vec!["USER\\DEV".into()], white: vec!["user.exe".into()], ..Default::default() };
        let before = ctl.clone();
        let applied = hide_with(&mut ctl, PAD, APP).unwrap();
        assert_eq!(ctl.black, vec!["USER\\DEV".to_string(), PAD.to_string()]);
        assert_eq!(ctl.white, vec!["user.exe".to_string(), APP.to_string()]);
        assert!(ctl.active && applied.activated);
        unhide_with(&mut ctl, &applied).unwrap();
        assert_eq!((ctl.black, ctl.white, ctl.active), (before.black, before.white, before.active));
    }

    #[test]
    fn entries_the_user_already_had_are_left_in_place() {
        let mut ctl = MockControl {
            black: vec![PAD.to_ascii_lowercase()],
            white: vec![APP.to_string()],
            active: true,
            inverse: false,
        };
        let applied = hide_with(&mut ctl, PAD, APP).unwrap();
        assert!(applied.is_empty());
        unhide_with(&mut ctl, &applied).unwrap();
        assert_eq!(ctl.black.len(), 1);
        assert_eq!(ctl.white.len(), 1);
        assert!(ctl.active);
    }

    #[test]
    fn inverse_mode_never_adds_periph_to_the_list() {
        let mut ctl = MockControl { inverse: true, ..Default::default() };
        let applied = hide_with(&mut ctl, PAD, APP).unwrap();
        assert!(ctl.white.is_empty());
        assert_eq!(applied.app, None);
    }

    #[test]
    fn ledger_round_trip() {
        let a = Applied { device: Some(PAD.into()), app: Some(APP.into()), activated: true };
        assert_eq!(Applied::from_ledger(&a.to_ledger()), a);
        assert!(Applied::from_ledger("garbage\n").is_empty());
    }

    /// Read-only check against the installed driver: `cargo test -p hidhide -- --ignored`.
    /// Only GET requests are sent; nothing is changed.
    #[test]
    #[ignore]
    #[cfg(windows)]
    fn reads_the_installed_hidhide_configuration() {
        let image = nt_image_path(&std::env::current_exe().unwrap()).unwrap();
        assert!(image.starts_with(r"\Device\"), "{image}");
        let Some(mut ctl) = open().unwrap() else {
            eprintln!("HidHide not installed");
            return;
        };
        let black = ctl.blacklist().unwrap();
        let white = ctl.whitelist().unwrap();
        eprintln!(
            "active={} inverse={} blacklist={} entries whitelist={} entries",
            ctl.active().unwrap(),
            ctl.inverse().unwrap(),
            black.len(),
            white.len()
        );
        assert!(white.iter().all(|w| w.starts_with(r"\Device\") || w.starts_with(r"\\")), "{white:?}");
    }

    #[test]
    fn nt_path_join() {
        assert_eq!(
            join_nt_path(r"\Device\HarddiskVolume3", r"C:\", r"C:\Apps\periph.exe").as_deref(),
            Some(r"\Device\HarddiskVolume3\Apps\periph.exe")
        );
        assert_eq!(
            join_nt_path(r"\Device\HarddiskVolume7\", r"D:\mnt\x\", r"d:\MNT\x\a.exe").as_deref(),
            Some(r"\Device\HarddiskVolume7\a.exe")
        );
        assert!(join_nt_path(r"\Device\X", r"D:\", r"C:\a.exe").is_none());
    }
}
