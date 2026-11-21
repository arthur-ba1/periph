//! `periph-pack new`: a package folder to start from, with the identification filled in and
//! the protocol left for the author (or the AI agent following the periph-device skill).

use anyhow::{bail, Result};
use serde_json::json;
use std::path::Path;

pub struct New<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub kind: &'a str,
    pub vid: u16,
    pub pid: u16,
    pub usage_page: u16,
    pub usage: u16,
    /// "registers" or "reports"
    pub transport: &'a str,
}

fn write(path: &Path, v: &serde_json::Value) -> Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(v)? + "\n")?;
    Ok(())
}

pub fn create(dir: &Path, n: &New) -> Result<()> {
    if !n.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') || n.id.is_empty() {
        bail!("id must be lowercase letters, digits and '-'");
    }
    if dir.exists() {
        bail!("{} already exists", dir.display());
    }
    std::fs::create_dir_all(dir.join("locales"))?;
    let hex4 = |v: u16| format!("0x{v:04X}");
    write(
        &dir.join("manifest.json"),
        &json!({
            "format": 1,
            "id": n.id,
            "name": n.name,
            "kind": n.kind,
            "version": "0.1.0",
            "authors": [],
            "description": "",
            "products": [{ "vid": hex4(n.vid), "pid": hex4(n.pid) }],
            "interfaces": { "command": { "usagePage": hex4(n.usage_page), "usage": hex4(n.usage) } }
        }),
    )?;
    let transport = match n.transport {
        "registers" => json!({
            "type": "registers",
            "interface": "command",
            "reportLength": 0,
            "read": "TODO: read request template, e.g. 06 A5 A4 {len} {addr:u16le} 00 00",
            "write": "TODO: write request template, e.g. 06 A5 A5 {len} {addr:u16le} 00 00 {data}",
            "maxChunk": 0,
            "response": { "reportId": 0, "status": 0, "data": 0, "ready": [] }
        }),
        _ => json!({
            "type": "reports",
            "interface": "command",
            "send": "output",
            "reportLength": 0
        }),
    };
    let mut device = json!({
        "transport": transport,
        "settings": [],
        "pages": [],
        "examples": [],
        "demo": { "config": {} }
    });
    if n.transport != "registers" {
        device["reads"] = json!([]);
    }
    write(&dir.join("device.json"), &device)?;
    write(&dir.join("locales").join("en.json"), &json!({ "pages": {}, "settings": {} }))?;
    Ok(())
}
