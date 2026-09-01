//! Environment snapshot, hygiene gate, wake-lock guard, record writer.

use crate::topology;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Battery {
    pub pct: Option<i64>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Env {
    pub epoch_secs: u64,
    pub kernel: String,
    pub load1: Option<f64>,
    pub loadavg_raw: String,
    pub mem_total_kb: Option<u64>,
    pub mem_avail_kb: Option<u64>,
    pub battery: Option<Battery>,
    pub cores: Vec<topology::Core>,
    pub rustc: String,
}

fn read_first(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.lines().next().map(String::from))
}

pub fn battery() -> Option<Battery> {
    let out = Command::new("termux-battery-status").output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    let pct = s
        .find("\"percentage\"")
        .and_then(|i| s[i..].split(':').nth(1))
        .and_then(|r| r.split(',').next())
        .and_then(|r| r.trim().parse::<i64>().ok());
    let status = s
        .find("\"status\"")
        .and_then(|i| s[i..].split(':').nth(1))
        .and_then(|r| r.split(',').next())
        .map(|x| x.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| "unknown".into());
    Some(Battery { pct, status })
}

pub fn snapshot() -> Env {
    let loadavg_raw = read_first("/proc/loadavg").unwrap_or_default();
    let load1 = loadavg_raw
        .split_whitespace()
        .next()
        .and_then(|t| t.parse::<f64>().ok());

    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut mem_total_kb = None;
    let mut mem_avail_kb = None;
    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            mem_total_kb = line["MemTotal:".len()..]
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse::<u64>()
                .ok();
        } else if line.starts_with("MemAvailable:") {
            mem_avail_kb = line["MemAvailable:".len()..]
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse::<u64>()
                .ok();
        }
    }

    Env {
        epoch_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        kernel: read_first("/proc/version").unwrap_or_default(),
        load1,
        loadavg_raw,
        mem_total_kb,
        mem_avail_kb,
        battery: battery(),
        cores: topology::read(),
        rustc: Command::new("rustc")
            .arg("--version")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".into()),
    }
}

/// Hygiene gate. Returns (abort_reasons, warnings).
pub fn hygiene() -> (Vec<String>, Vec<String>) {
    let mut aborts = Vec::new();
    let mut warns = Vec::new();
    let env = snapshot();
    if let Some(l) = env.load1 {
        if l >= 4.0 {
            aborts.push(format!("loadavg {l} — system busy"));
        } else if l >= 2.0 {
            warns.push(format!("loadavg {l}"));
        }
    }
    match &env.battery {
        Some(b) => {
            if let Some(p) = b.pct {
                if p < 50 {
                    aborts.push(format!("battery {p}% < 50%"));
                }
            }
            let stat = b.status.to_uppercase();
            if stat == "CHARGING" || stat == "FULL" {
                aborts.push("device on charger — thermal state invalid".into());
            } else if stat == "NOT_CHARGING" {
                warns.push("battery state NOT_CHARGING (plugged but idle)".into());
            }
        }
        None => warns.push("battery unknown (Termux:API not installed?)".into()),
    }
    if env.mem_avail_kb.map_or(false, |m| m < 512 * 1024) {
        aborts.push("MemAvailable < 512 MB".into());
    }
    (aborts, warns)
}

pub struct WakeLock;
impl WakeLock {
    pub fn acquire() -> WakeLock {
        let _ = Command::new("termux-wake-lock").status();
        WakeLock
    }
}
impl Drop for WakeLock {
    fn drop(&mut self) {
        let _ = Command::new("termux-wake-unlock").status();
    }
}

pub fn json_esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// Record discipline: results that aren't recorded didn't happen.
pub fn write_record(tag: &str, json: &str) -> std::io::Result<String> {
    let dir = "bench/records";
    std::fs::create_dir_all(dir)?;
    let path = format!(
        "{dir}/{tag}_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    std::fs::write(&path, json)?;
    Ok(path)
}
