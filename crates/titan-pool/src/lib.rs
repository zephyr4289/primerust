//! titan-pool: Lock-free heterogeneous siege engine for multi-core sieving.

pub mod pool;
pub mod telemetry;
pub mod unit;
pub mod worker;

use std::sync::Arc;
pub use telemetry::WorkerTelemetry;
pub use unit::WorkUnit;
pub use worker::PoolRunner;

/// Count primes <= n across all 8 cores using the heterogeneous pool.
#[inline]
pub fn pi_mt(n: u64) -> u64 {
    pi_mt_with_workers(n, 8)
}

/// Count primes <= n using a specified number of pinned workers.
#[inline]
pub fn pi_mt_with_workers(n: u64, num_workers: usize) -> u64 {
    let units = unit::generate_work_units(n, 64);
    let (count, _) = PoolRunner::run(n, num_workers, units);
    count
}

/// Count primes <= n returning both the count and telemetry slots.
#[inline]
pub fn pi_mt_full(
    n: u64,
    num_workers: usize,
    target_units: usize,
) -> (u64, Vec<Arc<WorkerTelemetry>>) {
    let units = unit::generate_work_units(n, target_units);
    PoolRunner::run(n, num_workers, units)
}
