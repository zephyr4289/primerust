//! Worker: pinned execution lane for physical sieving.

use crate::pool::WorkPool;
use crate::telemetry::WorkerTelemetry;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;
use titan_bench::pin;
use titan_sieve::arena::SieveArena;
use titan_sieve::segment::{count_primes_range_direct, count_primes_with_arena};

pub struct PoolRunner;

/// Explicit 8-core DynamIQ CPU pinning for Snapdragon 4 Gen 2 (SM4450).
/// Cores 0..=5: Cortex-A55 (Little, 1.96 GHz), Cores 6..=7: Cortex-A78 (Big, 2.21 GHz).
/// Falls back to cluster mask if per-core pin fails (Termux cpuset restrictions).
#[cfg(target_os = "android")]
pub fn bind_worker_affinity(thread_id: usize) {
    unsafe {
        let mut set: libc::cpu_set_t = core::mem::zeroed();
        libc::CPU_ZERO(&mut set);

        // Qualcomm Snapdragon 4 Gen 2 (SM4450) Topology:
        // Cores 0..=5: Cortex-A55 (Little cluster, 1.96 GHz, 32 KiB L1D, 128 KiB L2)
        // Cores 6..=7: Cortex-A78 (Big cluster, 2.21 GHz, 64 KiB L1D, 512 KiB L2)
        let target_cpu = match thread_id {
            0..=5 => thread_id,             // Pin worker 0..5 directly to Little cores 0..5
            6 => 6,                          // Pin worker 6 to Big Core 6
            7 => 7,                          // Pin worker 7 to Big Core 7
            _ => thread_id % 8,             // Wrap-around fallback
        };

        libc::CPU_SET(target_cpu, &mut set);

        let tid = libc::gettid();
        let ret = libc::sched_setaffinity(
            tid,
            core::mem::size_of::<libc::cpu_set_t>(),
            &set,
        );

        if ret != 0 {
            // Fallback: Bind to cluster mask if per-core pin fails
            let cluster_mask: usize = if thread_id >= 6 { 0xC0 } else { 0x3F };
            let mut fallback_set: libc::cpu_set_t = core::mem::zeroed();
            libc::CPU_ZERO(&mut fallback_set);
            for cpu in 0..8 {
                if (cluster_mask & (1 << cpu)) != 0 {
                    libc::CPU_SET(cpu, &mut fallback_set);
                }
            }
            libc::sched_setaffinity(tid, core::mem::size_of::<libc::cpu_set_t>(), &fallback_set);
        }
    }
}

#[cfg(not(target_os = "android"))]
pub fn bind_worker_affinity(thread_id: usize) {
    // CI-1 SMP: round-robin across detected ncpu (4 on free CI).
    // Best-effort: ignore failure (restricted cpusets on shared runners).
    let target = titan_core::cpu::pin_target(thread_id);
    unsafe {
        let mut set: libc::cpu_set_t = core::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        libc::CPU_SET(target, &mut set);
        libc::sched_setaffinity(0, core::mem::size_of::<libc::cpu_set_t>(), &set);
    }
}

impl PoolRunner {
    /// Execute multi-threaded sieve on N up to num_workers (<= 8).
    pub fn run(
        n: u64,
        num_workers: usize,
        units: Vec<crate::unit::WorkUnit>,
    ) -> (u64, Vec<Arc<WorkerTelemetry>>) {
        let num_workers = num_workers.clamp(1, 8);
        let pool = Arc::new(WorkPool::new(units));
        let barrier = Arc::new(Barrier::new(num_workers));

        let mut telemetries = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            telemetries.push(Arc::new(WorkerTelemetry::new()));
        }

        // Assigned CPUs: big cores first if few workers, else explicit DynamIQ map.
        // FIX (Phase 9.2.0): old `num <= 6 => 0..num` starved Big cores 6,7.
        let assigned_cpus: Vec<usize> = if num_workers == 1 {
            vec![6] // single big core
        } else if num_workers == 2 {
            vec![6, 7] // two big cores
        } else if num_workers <= 6 {
            // Include both Big cores first, then Littles (never starve A78s)
            let mut cpus = vec![6, 7];
            cpus.extend(0..(num_workers - 2));
            cpus
        } else {
            // 7 or 8 workers: full SoC
            (0..num_workers).collect()
        };

        let mut handles = Vec::with_capacity(num_workers - 1);

        // Spawn workers 0..(num_workers - 1)
        for worker_id in 0..(num_workers - 1) {
            let cpu = assigned_cpus[worker_id];
            let pool_clone = Arc::clone(&pool);
            let barrier_clone = Arc::clone(&barrier);
            let telemetry_clone = Arc::clone(&telemetries[worker_id]);

            let handle = thread::spawn(move || {
                // 1. Self-pin and publish CPU
                let _ = pin::set_affinity(cpu);
                telemetry_clone.publish_cpu(cpu);

                // 2. Core geometry: 32 KiB matches both A78 and A55 L1D on SM4450
                let seg_sz = 32768;
                let mut arena = SieveArena::new(n, seg_sz);

                // 3. Wait at start barrier
                barrier_clone.wait();

                // 4. Unit pull loop
                let mut local_count = 0u64;
                while let Some(unit) = pool_clone.pull() {
                    let t0 = Instant::now();
                    let count = if unit.lo == 0 {
                        count_primes_with_arena(unit.hi, seg_sz, &mut arena)
                    } else {
                        count_primes_range_direct(unit.lo, unit.hi, seg_sz, &mut arena)
                    };
                    let dur = t0.elapsed().as_nanos() as u64;
                    local_count += count;
                    telemetry_clone.record_unit(count, dur);
                }

                local_count
            });
            handles.push(handle);
        }

        // Main thread executes the last worker lane
        let main_worker_id = num_workers - 1;
        let main_cpu = assigned_cpus[main_worker_id];
        let _ = pin::set_affinity(main_cpu);
        telemetries[main_worker_id].publish_cpu(main_cpu);

        let main_seg_sz = 32768;
        let mut main_arena = SieveArena::new(n, main_seg_sz);

        // Wait at start barrier
        barrier.wait();

        let mut main_count = 0u64;
        while let Some(unit) = pool.pull() {
            let t0 = Instant::now();
            let count = if unit.lo == 0 {
                count_primes_with_arena(unit.hi, main_seg_sz, &mut main_arena)
            } else {
                count_primes_range_direct(unit.lo, unit.hi, main_seg_sz, &mut main_arena)
            };
            let dur = t0.elapsed().as_nanos() as u64;
            main_count += count;
            telemetries[main_worker_id].record_unit(count, dur);
        }

        // Aggregate counts
        let mut total_primes = main_count;
        for handle in handles {
            total_primes += handle.join().expect("Worker thread panicked");
        }

        let _ = pin::set_full_affinity();
        (total_primes, telemetries)
    }
}
