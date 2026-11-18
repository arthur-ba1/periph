//! QMK/VIA raw HID client.
//!
//! Every exchange is a 32-byte output report (sent with report id 0) answered by a
//! 32-byte input report that echoes the command id. `0xFF` back means "unhandled".
//! Reference: QMK `quantum/via.h` (protocol 0x000C here).

use anyhow::{bail, Context, Result};
use hidapi::{HidApi, HidDevice};

pub const USAGE_PAGE: u16 = 0xFF60;
pub const USAGE: u16 = 0x61;

mod id {
    pub const GET_PROTOCOL_VERSION: u8 = 0x01;
    pub const GET_KEYBOARD_VALUE: u8 = 0x02;
    pub const DYNAMIC_KEYMAP_GET_KEYCODE: u8 = 0x04;
    pub const DYNAMIC_KEYMAP_SET_KEYCODE: u8 = 0x05;
    pub const CUSTOM_SET_VALUE: u8 = 0x07;
    pub const CUSTOM_GET_VALUE: u8 = 0x08;
    pub const CUSTOM_SAVE: u8 = 0x09;
    pub const DYNAMIC_KEYMAP_MACRO_GET_COUNT: u8 = 0x0C;
    pub const DYNAMIC_KEYMAP_GET_LAYER_COUNT: u8 = 0x11;
    pub const DYNAMIC_KEYMAP_GET_BUFFER: u8 = 0x12;
    pub const DYNAMIC_KEYMAP_GET_ENCODER: u8 = 0x14;
    pub const DYNAMIC_KEYMAP_SET_ENCODER: u8 = 0x15;
    pub const UNHANDLED: u8 = 0xFF;
}

/// Keycodes that would wipe or reflash the board; never written by this client.
pub fn is_dangerous_keycode(k: u16) -> bool {
    // QK_BOOTLOADER, QK_REBOOT, QK_DEBUG_TOGGLE, QK_CLEAR_EEPROM, QK_MAKE
    matches!(k, 0x7C00..=0x7C03 | 0x7C13)
}

pub struct Keyboard {
    dev: HidDevice,
    pub vendor_id: u16,
    pub product_id: u16,
    pub product: String,
    pub manufacturer: String,
}

impl Keyboard {
    /// Open the first VIA-capable keyboard (raw HID usage page 0xFF60 / usage 0x61).
    pub fn open_any() -> Result<Self> {
        let api = HidApi::new()?;
        let info = api
            .device_list()
            .find(|d| d.usage_page() == USAGE_PAGE && d.usage() == USAGE)
            .context("nenhum teclado VIA encontrado")?;
        Ok(Self {
            dev: info.open_device(&api).context("open VIA interface")?,
            vendor_id: info.vendor_id(),
            product_id: info.product_id(),
            product: info.product_string().unwrap_or_default().to_string(),
            manufacturer: info.manufacturer_string().unwrap_or_default().to_string(),
        })
    }

    fn query(&self, payload: &[u8]) -> Result<[u8; 32]> {
        let mut buf = [0u8; 32];
        // Windows delivers raw-HID replies to every open handle, so replies to other VIA
        // clients (usevia.app, other tools) pile up in our queue. Drop them first.
        while self.dev.read_timeout(&mut buf, 0)? > 0 {}

        let mut out = [0u8; 33];
        out[1..1 + payload.len()].copy_from_slice(payload);
        self.dev.write(&out).context("hid write")?;
        // VIA echoes the command id and its address parameters in place; require the
        // echo to match (id + up to 3 params) so a foreign reply is never taken as ours.
        let echo = payload.len().min(4);
        for _ in 0..16 {
            let n = self.dev.read_timeout(&mut buf, 500)?;
            if n == 0 {
                bail!("o teclado não respondeu ao comando 0x{:02X}", payload[0]);
            }
            // QMK only overwrites byte 0 with 0xFF; the params stay in place
            if buf[0] == id::UNHANDLED && buf[1..echo] == payload[1..echo] {
                bail!("comando 0x{:02X} não suportado pelo firmware", payload[0]);
            }
            if buf[..echo] == payload[..echo] {
                return Ok(buf);
            }
        }
        bail!("resposta inesperada ao comando 0x{:02X} (outro programa usando o teclado?)", payload[0])
    }

    pub fn protocol_version(&self) -> Result<u16> {
        let r = self.query(&[id::GET_PROTOCOL_VERSION])?;
        Ok(u16::from_be_bytes([r[1], r[2]]))
    }

    pub fn uptime_ms(&self) -> Result<u32> {
        let r = self.query(&[id::GET_KEYBOARD_VALUE, 0x01])?;
        Ok(u32::from_be_bytes([r[2], r[3], r[4], r[5]]))
    }

    pub fn layer_count(&self) -> Result<u8> {
        Ok(self.query(&[id::DYNAMIC_KEYMAP_GET_LAYER_COUNT])?[1])
    }

    pub fn macro_count(&self) -> Result<u8> {
        Ok(self.query(&[id::DYNAMIC_KEYMAP_MACRO_GET_COUNT])?[1])
    }

    /// Whole keymap as `[layer][row][col]` keycodes.
    pub fn keymap(&self, layers: u8, rows: u8, cols: u8) -> Result<Vec<Vec<Vec<u16>>>> {
        let total = layers as usize * rows as usize * cols as usize * 2;
        let mut raw = Vec::with_capacity(total);
        let mut off = 0usize;
        while off < total {
            let size = (total - off).min(28) as u8;
            let r = self.query(&[id::DYNAMIC_KEYMAP_GET_BUFFER, (off >> 8) as u8, off as u8, size])?;
            raw.extend_from_slice(&r[4..4 + size as usize]);
            off += size as usize;
        }
        let at = |l: usize, r: usize, c: usize| {
            let i = ((l * rows as usize + r) * cols as usize + c) * 2;
            u16::from_be_bytes([raw[i], raw[i + 1]])
        };
        Ok((0..layers as usize)
            .map(|l| (0..rows as usize).map(|r| (0..cols as usize).map(|c| at(l, r, c)).collect()).collect())
            .collect())
    }

    pub fn keycode(&self, layer: u8, row: u8, col: u8) -> Result<u16> {
        let r = self.query(&[id::DYNAMIC_KEYMAP_GET_KEYCODE, layer, row, col])?;
        Ok(u16::from_be_bytes([r[4], r[5]]))
    }

    /// Set a keycode and read it back to confirm.
    pub fn set_keycode(&self, layer: u8, row: u8, col: u8, code: u16) -> Result<()> {
        if is_dangerous_keycode(code) {
            bail!("keycode 0x{code:04X} bloqueado (bootloader/limpar EEPROM)");
        }
        let [hi, lo] = code.to_be_bytes();
        self.query(&[id::DYNAMIC_KEYMAP_SET_KEYCODE, layer, row, col, hi, lo])?;
        let back = self.keycode(layer, row, col)?;
        if back != code {
            bail!("o teclado guardou 0x{back:04X} em vez de 0x{code:04X}");
        }
        Ok(())
    }

    /// `clockwise`: false = counter-clockwise slot, true = clockwise slot.
    pub fn encoder(&self, layer: u8, encoder: u8, clockwise: bool) -> Result<u16> {
        let r = self.query(&[id::DYNAMIC_KEYMAP_GET_ENCODER, layer, encoder, clockwise as u8])?;
        Ok(u16::from_be_bytes([r[4], r[5]]))
    }

    pub fn set_encoder(&self, layer: u8, encoder: u8, clockwise: bool, code: u16) -> Result<()> {
        if is_dangerous_keycode(code) {
            bail!("keycode 0x{code:04X} bloqueado (bootloader/limpar EEPROM)");
        }
        let [hi, lo] = code.to_be_bytes();
        self.query(&[id::DYNAMIC_KEYMAP_SET_ENCODER, layer, encoder, clockwise as u8, hi, lo])?;
        let back = self.encoder(layer, encoder, clockwise)?;
        if back != code {
            bail!("o encoder guardou 0x{back:04X} em vez de 0x{code:04X}");
        }
        Ok(())
    }

    /// Custom menu value (VIA "content": [name, channel, id]). Returns the value bytes.
    pub fn custom_get(&self, channel: u8, value_id: u8, len: usize) -> Result<Vec<u8>> {
        let r = self.query(&[id::CUSTOM_GET_VALUE, channel, value_id])?;
        Ok(r[3..3 + len.min(29)].to_vec())
    }

    pub fn custom_set(&self, channel: u8, value_id: u8, data: &[u8]) -> Result<()> {
        let mut p = vec![id::CUSTOM_SET_VALUE, channel, value_id];
        p.extend_from_slice(&data[..data.len().min(29)]);
        self.query(&p)?;
        Ok(())
    }

    /// Persist a custom channel to EEPROM (what VIA does after changing lighting).
    pub fn custom_save(&self, channel: u8) -> Result<()> {
        self.query(&[id::CUSTOM_SAVE, channel])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dangerous_keycodes_blocked() {
        assert!(super::is_dangerous_keycode(0x7C00));
        assert!(super::is_dangerous_keycode(0x7C03));
        assert!(!super::is_dangerous_keycode(0x0004));
        assert!(!super::is_dangerous_keycode(0x5222));
    }
}
