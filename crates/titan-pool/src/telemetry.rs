//! Telemetry: 128-byte padded per-worker in-situ telemetry slots.
//!
//! Prevents false sharing across core caches and provides lock-free
//! observational data of worker progress and thermal frequency derating.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[repr(C, align(128))]
pub struct WorkerTelemetry {
    pub cpu_id: AtomicUsize,
    pub units_completed: AtomicUsize,
    pub primes_tallied: AtomicU64,
    pub total_time_ns: AtomicU64,
    pub last_unit_time_ns: AtomicU64,
    _pad: [u8; 128 - 40],
}

impl WorkerTelemetry {
    pub fn new() -> Self {
        Self {
            cpu_id: AtomicUsize::new(usize::MAX),
            units_completed: AtomicUsize::new(0),
            primes_tallied: AtomicU64::new(0),
            total_time_ns: AtomicU64::new(0),
            last_unit_time_ns: AtomicU64::new(0),
            _pad: [0; 128 - 40],
        }
    }

    pub fn publish_cpu(&self, cpu: usize) {
        self.cpu_id.store(cpu, Ordering::Release);
    }

    pub fn record_unit(&self, primes: u64, duration_ns: u64) {
        self.units_completed.fetch_add(1, Ordering::Relaxed);
        self.primes_tallied.fetch_add(primes, Ordering::Relaxed);
        self.total_time_ns.fetch_add(duration_ns, Ordering::Relaxed);
        self.last_unit_time_ns.store(duration_ns, Ordering::Release);
    }

    pub fn snapshot(&self) -> (usize, usize, u64, u64) {
        (
            self.cpu_id.load(Ordering::Acquire),
            self.units_completed.load(Ordering::Relaxed),
            self.primes_tallied.load(Ordering::Relaxed),
            self.total_time_ns.load(Ordering::Relaxed),
        )
    }
}
