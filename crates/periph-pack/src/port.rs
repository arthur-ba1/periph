//! What the engine talks to: one HID interface (top-level collection) of the device.
//! `HidPort` is the real thing; `SimRegisters` is a register map in memory, used by
//! `periph-pack check` and the tests.

use crate::engine::request;
use crate::spec::{ReportKind, Registers, Reports};
use std::collections::VecDeque;
use anyhow::{Context, Result};
use std::collections::BTreeMap;

pub trait Port: Send {
    /// Send an output report (first byte = report id).
    fn write(&mut self, report: &[u8]) -> Result<()>;
    /// Get input report `id` (at most `len` bytes, report id included).
    fn input_report(&mut self, id: u8, len: usize) -> Result<Vec<u8>>;
    /// Next input report the device sends on its own, or `None` after `timeout_ms`.
    fn read(&mut self, timeout_ms: i32) -> Result<Option<Vec<u8>>>;
    /// Send a feature report (first byte = report id).
    fn send_feature(&mut self, _report: &[u8]) -> Result<()> {
        anyhow::bail!("feature reports are not supported here")
    }
    /// Get feature report `id` (at most `len` bytes, report id included).
    fn get_feature(&mut self, _id: u8, _len: usize) -> Result<Vec<u8>> {
        anyhow::bail!("feature reports are not supported here")
    }
}

pub struct HidPort(pub hidapi::HidDevice);

impl Port for HidPort {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        self.0.write(report).context("hid write")?;
        Ok(())
    }
    fn input_report(&mut self, id: u8, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        buf[0] = id;
        // hidapi on Windows can report len+1 here; never trust it beyond the buffer.
        let n = self.0.get_input_report(&mut buf).context("get input report")?.min(len);
        buf.truncate(n);
        Ok(buf)
    }
    fn read(&mut self, timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        let mut buf = [0u8; 1024];
        let n = self.0.read_timeout(&mut buf, timeout_ms)?;
        Ok((n > 0).then(|| buf[..n].to_vec()))
    }
    fn send_feature(&mut self, report: &[u8]) -> Result<()> {
        self.0.send_feature_report(report).context("hid send feature report")?;
        Ok(())
    }
    fn get_feature(&mut self, id: u8, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        buf[0] = id;
        let n = self.0.get_feature_report(&mut buf).context("get feature report")?.min(len);
        buf.truncate(n);
        Ok(buf)
    }
}

/// A device that answers a `registers` transport from memory. Records every request.
pub struct SimRegisters {
    spec: Registers,
    pub memory: BTreeMap<u32, u8>,
    pub requests: Vec<Vec<u8>>,
    answer: Vec<u8>,
}

impl SimRegisters {
    pub fn new(spec: &Registers) -> Self {
        SimRegisters { spec: spec.clone(), memory: BTreeMap::new(), requests: Vec::new(), answer: Vec::new() }
    }

    pub fn load(&mut self, addr: u32, bytes: &[u8]) {
        for (i, b) in bytes.iter().enumerate() {
            self.memory.insert(addr + i as u32, *b);
        }
    }

    /// The write requests seen so far (read requests left out).
    pub fn writes(&self) -> Vec<&Vec<u8>> {
        self.requests.iter().filter(|r| self.spec.write.matches(r).is_some() && self.spec.read.matches(r).is_none()).collect()
    }

    fn reply(&mut self, data: &[u8]) {
        let r = &self.spec.response;
        // Answers carry the report id here; real devices may leave it out.
        let mut a = vec![0u8; self.spec.report_length.max(r.data + 1 + data.len())];
        a[0] = r.report_id;
        a[1 + r.status] = r.ready[0];
        a[1 + r.data..1 + r.data + data.len()].copy_from_slice(data);
        self.answer = a;
    }
}

impl Port for SimRegisters {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        self.requests.push(report.to_vec());
        let var = |vars: &[(String, u32)], n: &str| vars.iter().find(|(k, _)| k == n).map(|(_, v)| *v);
        if let Some((vars, _)) = self.spec.read.matches(report) {
            let (addr, len) = (var(&vars, "addr").unwrap_or(0), var(&vars, "len").unwrap_or(0));
            let data: Vec<u8> = (0..len).map(|i| self.memory.get(&(addr + i)).copied().unwrap_or(0)).collect();
            self.reply(&data);
        } else if let Some((vars, data)) = self.spec.write.matches(report) {
            self.load(var(&vars, "addr").unwrap_or(0), &data);
            self.reply(&[]);
        } else {
            anyhow::bail!("simulated device: request matches neither template: {report:02X?}");
        }
        Ok(())
    }
    fn input_report(&mut self, _id: u8, _len: usize) -> Result<Vec<u8>> {
        Ok(self.answer.clone())
    }
    fn read(&mut self, _timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
}

/// A device that answers a `reports` transport with canned answers (one per read), and
/// records every request it receives.
pub struct SimReports {
    spec: Reports,
    /// read request bytes -> (answer kind, full answer report)
    reads: Vec<(Vec<u8>, ReportKind, Vec<u8>)>,
    pub requests: Vec<Vec<u8>>,
    feature: Option<Vec<u8>>,
    input: VecDeque<Vec<u8>>,
}

impl SimReports {
    /// `answers`: read id -> answer payload (after the report id).
    pub fn new(pkg: &crate::Package, answers: &BTreeMap<String, Vec<u8>>) -> Result<Self> {
        let crate::spec::Transport::Reports(spec) = &pkg.device.transport else { anyhow::bail!("not a reports transport") };
        let mut reads = Vec::new();
        for r in &pkg.device.reads {
            let req = request(spec, &r.request, None, &[])?;
            let mut ans = vec![r.response.report_id];
            ans.extend_from_slice(answers.get(&r.id).map(Vec::as_slice).unwrap_or_default());
            if let Some(len) = r.response.length {
                ans.resize(ans.len().max(len), 0);
            }
            reads.push((req, r.response.from, ans));
        }
        Ok(SimReports { spec: spec.clone(), reads, requests: Vec::new(), feature: None, input: VecDeque::new() })
    }

    fn got(&mut self, report: &[u8]) {
        self.requests.push(report.to_vec());
        if let Some((_, kind, ans)) = self.reads.iter().find(|(req, _, _)| req == report) {
            match kind {
                ReportKind::Feature => self.feature = Some(ans.clone()),
                _ => self.input.push_back(ans.clone()),
            }
        }
    }

    /// The requests that were not reads, in order.
    pub fn writes(&self) -> Vec<&Vec<u8>> {
        self.requests.iter().filter(|r| !self.reads.iter().any(|(req, _, _)| req == *r)).collect()
    }
}

impl Port for SimReports {
    fn write(&mut self, report: &[u8]) -> Result<()> {
        anyhow::ensure!(self.spec.send != ReportKind::Feature, "this package sends feature reports");
        self.got(report);
        Ok(())
    }
    fn send_feature(&mut self, report: &[u8]) -> Result<()> {
        anyhow::ensure!(self.spec.send == ReportKind::Feature, "this package sends output reports");
        self.got(report);
        Ok(())
    }
    fn get_feature(&mut self, id: u8, _len: usize) -> Result<Vec<u8>> {
        Ok(self.feature.clone().unwrap_or_else(|| vec![id]))
    }
    fn input_report(&mut self, id: u8, _len: usize) -> Result<Vec<u8>> {
        Ok(vec![id])
    }
    fn read(&mut self, _timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        Ok(self.input.pop_front())
    }
}
