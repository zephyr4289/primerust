//! Phase 32: Per-Phase High-Resolution Nanosecond Timers.
//!
//! Uses std::time::Instant (backed by vDSO cntvct on ARM64 Linux).
//! Thread-local recording, aggregated at join in thread-id order (deterministic).

use std::time::Instant;

pub const PHASES: [&str; 8] = [
    "boot_sieve",
    "b_mark",
    "b_count_resolve",
    "ftd_build",
    "d_walk",
    "sigma_ac",
    "combine_alloc",
    "total",
];

#[derive(Clone, Debug)]
pub struct PhaseTimers {
    starts: Vec<Option<Instant>>,
    pub sums_ns: [u128; 8],
}

impl PhaseTimers {
    pub fn new() -> Self {
        Self {
            starts: vec![None; 8],
            sums_ns: [0; 8],
        }
    }

    #[inline(always)]
    pub fn enter(&mut self, p: usize) {
        if p < 8 {
            self.starts[p] = Some(Instant::now());
        }
    }

    #[inline(always)]
    pub fn exit(&mut self, p: usize) {
        if p < 8 {
            if let Some(t0) = self.starts[p].take() {
                self.sums_ns[p] += t0.elapsed().as_nanos();
            }
        }
    }

    pub fn merge(&mut self, other: &PhaseTimers) {
        for i in 0..8 {
            self.sums_ns[i] += other.sums_ns[i];
        }
    }

    pub fn report(&self, model_ms: [f64; 8]) -> String {
        PHASES
            .iter()
            .zip(self.sums_ns)
            .zip(model_ms)
            .map(|((name, ns), m)| {
                let ms = ns as f64 / 1e6;
                let d = if m > 0.0 { 100.0 * (ms - m) / m } else { 0.0 };
                format!(
                    "{:<16} {:>9.2} ms  model {:>7.2}  Δ{:+6.1}%  {}",
                    name,
                    ms,
                    m,
                    d,
                    if d > 25.0 {
                        "RE-DERIVE CONSTANT"
                    } else if d < -25.0 {
                        "MODEL STALE"
                    } else {
                        "ok"
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Reads current process VmHWM (Peak Resident Set Size) in bytes from /proc/self/status.
pub fn vmhwm_bytes() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
        .map(|kb| kb * 1024)
        .unwrap_or(0)
}
