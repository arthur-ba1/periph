//! A package as a Periph [`Driver`]: opens the HID interfaces the manifest names and runs the
//! engine on them.

use crate::engine::{self, Cmd, Engine, State};
use crate::package::Package;
use crate::port::{HidPort, Port};
use anyhow::Result;
use periph_core::{t, tf, Absent, Connected, DeviceInfo, Driver, Session, Snapshot, Tray, TrayItem};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct PackageDriver {
    pub pkg: Arc<Package>,
    id: &'static str,
    name: &'static str,
}

impl PackageDriver {
    pub fn new(pkg: Package) -> Self {
        // DeviceInfo wants 'static strings; packages are loaded once, at startup.
        let id = Box::leak(pkg.manifest.id.clone().into_boxed_str());
        let name = Box::leak(pkg.manifest.name.clone().into_boxed_str());
        PackageDriver { pkg: Arc::new(pkg), id, name }
    }
}

struct PackageSession {
    pkg: Arc<Package>,
    state: State,
    cmd: Box<dyn Port>,
    status: Option<Box<dyn Port>>,
}

/// Open the first product of the manifest that is present, with every interface it names.
fn open_ports(pkg: &Package) -> Result<(BTreeMap<String, Box<dyn Port>>, Connected)> {
    let api = hidapi::HidApi::new()?;
    let m = &pkg.manifest;
    'products: for prod in &m.products {
        let mut ports: BTreeMap<String, Box<dyn Port>> = BTreeMap::new();
        let mut info = Connected::default();
        for (name, want) in &m.interfaces {
            let found = api.device_list().find(|d| {
                d.vendor_id() == prod.vid && d.product_id() == prod.pid && d.usage_page() == want.usage_page && d.usage() == want.usage
            });
            match found.map(|d| (d, d.open_device(&api))) {
                Some((d, Ok(dev))) => {
                    if info.product.is_empty() {
                        info = Connected {
                            product: d.product_string().unwrap_or_default().to_string(),
                            serial: d.serial_number().unwrap_or_default().to_string(),
                        };
                    }
                    ports.insert(name.clone(), Box::new(HidPort(dev)));
                }
                _ if want.optional => {}
                _ => continue 'products,
            }
        }
        return Ok((ports, info));
    }
    Err(Absent::new("notFound", tf("packages.errors.notFound", &[("name", &m.name)])).into())
}

impl Driver for PackageDriver {
    fn info(&self) -> DeviceInfo {
        DeviceInfo { id: self.id, name: self.name, kind: self.pkg.manifest.kind.into() }
    }

    fn open(&self) -> Result<(Box<dyn Session>, Connected)> {
        let (mut ports, info) = open_ports(&self.pkg)?;
        let iface = self.pkg.device.transport.interface();
        let cmd = ports.remove(iface).expect("transport interface is required");
        // The status may come on the command interface itself.
        let status = self.pkg.device.status.as_ref().filter(|s| s.interface != iface).and_then(|s| ports.remove(&s.interface));
        Ok((Box::new(PackageSession { pkg: self.pkg.clone(), state: State::default(), cmd, status }), info))
    }

    fn tray(&self, snap: &Snapshot) -> Option<Tray> {
        tray(&self.pkg, snap)
    }

    fn tray_command(&self, id: &str, snap: &Snapshot) -> Option<Value> {
        tray_command(&self.pkg, id, snap)
    }
}

impl Session for PackageSession {
    fn read_config(&mut self) -> Result<Value> {
        Engine::new(&self.pkg, self.cmd.as_mut(), &mut self.state).read_config()
    }

    fn apply(&mut self, cmd: Value) -> Result<()> {
        let cmd: Cmd = periph_core::parse(cmd)?;
        Engine::new(&self.pkg, self.cmd.as_mut(), &mut self.state).apply(cmd)
    }

    fn poll(&mut self) -> Result<Option<Value>> {
        let Some(st) = self.pkg.device.status.as_ref() else { return Ok(None) };
        let port = match self.status.as_mut() {
            Some(p) => p,
            None if st.interface == self.pkg.device.transport.interface() => &mut self.cmd,
            None => return Ok(None),
        };
        let end = Instant::now() + Duration::from_millis(st.timeout_ms);
        while Instant::now() < end {
            if let Some(buf) = port.read(50)? {
                if let Some(v) = engine::parse_status(&self.pkg, &buf) {
                    return Ok(Some(v));
                }
            }
        }
        Ok(None)
    }
}

fn key(pkg: &Package, k: &str) -> String {
    format!("{}.{k}", pkg.manifest.id)
}

/// Translated text, or `None` when the package has no such key.
fn text(pkg: &Package, k: &str) -> Option<String> {
    let full = key(pkg, k);
    let s = t(&full);
    (s != full).then_some(s)
}

fn show(v: &Value, unit: Option<&str>) -> String {
    let s = match v {
        Value::String(s) => s.clone(),
        v => v.to_string(),
    };
    match unit {
        Some(u) => format!("{s} {u}"),
        None => s,
    }
}

pub fn tray(pkg: &Package, snap: &Snapshot) -> Option<Tray> {
    let spec = pkg.device.tray.as_ref()?;
    let status = snap.status.as_ref();
    let battery = spec.battery.as_ref().and_then(|b| {
        let level = status?.get(&b.level)?.as_u64()?.min(100) as u8;
        let charging = b.charging.as_ref().and_then(|c| status?.get(c)?.as_bool()).unwrap_or(false);
        Some((level, charging))
    });
    let header = match (status, &spec.header) {
        (Some(Value::Object(st)), Some(h)) => {
            let args: Vec<(String, String)> = st.iter().map(|(k, v)| (k.clone(), show(v, None))).collect();
            let refs: Vec<(&str, &dyn std::fmt::Display)> = args.iter().map(|(k, v)| (k.as_str(), v as &dyn std::fmt::Display)).collect();
            tf(&key(pkg, h), &refs)
        }
        _ => pkg.manifest.name.clone(),
    };
    let mut items = Vec::new();
    if let Some(cfg) = &snap.config {
        for it in &spec.items {
            if it.refresh {
                items.push(TrayItem::Action { id: "refresh".into(), label: t("packages.refresh") });
                continue;
            }
            let Some(s) = it.setting.as_deref().and_then(|s| pkg.setting(s)) else { continue };
            let label = it.label.as_deref().map(|l| t(&key(pkg, l))).unwrap_or_else(|| t(&key(pkg, &format!("settings.{}.label", s.id))));
            let cur = &cfg[&s.id];
            let Ok(r) = pkg.resolve(&s.field) else { continue };
            if r.ty.is_bool() {
                items.push(TrayItem::Check { id: format!("set:{}", s.id), label, checked: cur.as_bool().unwrap_or(false) });
            } else if let Some(of) = s.index_of.as_deref().and_then(|o| pkg.setting(o)) {
                let arr = cfg[&of.id].as_array().cloned().unwrap_or_default();
                let n = of.count_from.as_ref().and_then(|c| cfg[c].as_u64()).map_or(arr.len(), |n| (n as usize).min(arr.len()));
                let unit = of.field.unit.as_deref();
                let options = (0..n)
                    .map(|i| TrayItem::Check { id: format!("set:{}:{i}", s.id), label: show(&arr[i], unit), checked: cur.as_u64() == Some(i as u64) })
                    .collect();
                items.push(TrayItem::Submenu { label, items: options });
            } else if let Some(values) = r.values {
                let options = values
                    .iter()
                    .enumerate()
                    .map(|(i, m)| TrayItem::Check {
                        id: format!("set:{}:{i}", s.id),
                        label: text(pkg, &format!("settings.{}.options.{}", s.id, show(&m.value, None)))
                            .unwrap_or_else(|| show(&m.value, s.field.unit.as_deref())),
                        checked: crate::value::same(&m.value, cur),
                    })
                    .collect();
                items.push(TrayItem::Submenu { label, items: options });
            }
        }
    }
    Some(Tray { header, battery, items })
}

pub fn tray_command(pkg: &Package, id: &str, snap: &Snapshot) -> Option<Value> {
    if id == "refresh" {
        return Some(json!({ "type": "refresh" }));
    }
    let rest = id.strip_prefix("set:")?;
    let (sid, opt) = match rest.split_once(':') {
        Some((s, o)) => (s, Some(o.parse::<usize>().ok()?)),
        None => (rest, None),
    };
    let s = pkg.setting(sid)?;
    let r = pkg.resolve(&s.field).ok()?;
    let value = match opt {
        None if r.ty.is_bool() => Value::Bool(!snap.config.as_ref()?[sid].as_bool()?),
        Some(i) if s.index_of.is_some() => json!(i),
        Some(i) => r.values?.get(i)?.value.clone(),
        None => return None,
    };
    Some(json!({ "type": "set", "id": sid, "value": value }))
}
