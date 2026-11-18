//! via-set: write one VIA custom value (e.g. RGB brightness) and save it to EEPROM.
//! Usage: via-set <channel> <id> <byte> [byte...]
//! Example (AKKO 5075 brightness = 198): via-set 3 1 198

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let args: Vec<u8> = std::env::args().skip(1).map(|a| a.parse()).collect::<Result<_, _>>()?;
    let [channel, id, data @ ..] = args.as_slice() else { bail!("usage: via-set <channel> <id> <byte>...") };
    if data.is_empty() {
        bail!("usage: via-set <channel> <id> <byte>...");
    }
    let kb = via::Keyboard::open_any()?;
    kb.custom_set(*channel, *id, data)?;
    kb.custom_save(*channel)?;
    let back = kb.custom_get(*channel, *id, data.len())?;
    println!("{} ch {channel} id {id}: wrote {data:?}, reads back {back:?}", kb.product);
    if back != data {
        bail!("value did not stick");
    }
    Ok(())
}
