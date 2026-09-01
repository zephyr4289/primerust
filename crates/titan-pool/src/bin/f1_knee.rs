//! Pre-Flight Experiment F1: DRAM Bandwidth Knee Curve.
//!
//! Synthesizes the real-mix memory traffic (streaming sequential reads + scattered 8B writes
//! + L1 segment RMW) across k = 1..8 workers to identify the LPDDR4X bandwidth saturation knee.

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;
use titan_bench::{pin, snapshot};

const STREAM_BYTES: usize = 8 * 1024 * 1024; // 8 MiB streaming buffer (exceeds all CPU caches)
const L1_BYTES: usize = 64 * 1024;            // 64 KiB L1 segment buffer
const ITERS: usize = 40;                      // 40 passes = ~320 MiB per worker

fn run_worker_stream(cpu: usize, barrier: Arc<Barrier>) -> f64 {
    pin::set_affinity(cpu).expect("Worker self-pin");
    let mut stream_buf = vec![0u8; STREAM_BYTES];
    let mut l1_buf = vec![0u8; L1_BYTES];

    // Warm-up
    stream_buf.fill(0x55);
    l1_buf.fill(0xFF);

    barrier.wait();
    let t0 = Instant::now();

    for pass in 0..ITERS {
        // 1. Sequential stream read (matches medium-state sequential streaming)
        let mut sum = 0u64;
        for chunk in stream_buf.chunks_exact(64) {
            let val = u64::from_ne_bytes(chunk[..8].try_into().unwrap());
            sum = sum.wrapping_add(val);
        }
        std::hint::black_box(sum);

        // 2. Scattered 8B writes into L1 buffer (matches bucket drain RMW)
        let mask = (pass as u8) | 1;
        for step in (0..L1_BYTES).step_by(128) {
            l1_buf[step] ^= mask;
        }
    }

    let elapsed = t0.elapsed().as_secs_f64();
    let bytes_transferred = (STREAM_BYTES * ITERS) as f64;
    bytes_transferred / elapsed / 1e9 // GB/s
}

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== PRE-FLIGHT EXPERIMENT F1: DRAM BANDWIDTH KNEE CURVE ==");

    for &k in &[1, 2, 4, 6, 8] {
        let barrier = Arc::new(Barrier::new(k));
        let mut handles = Vec::with_capacity(k);

        let assigned_cpus: Vec<usize> = if k == 1 {
            vec![6]
        } else if k == 2 {
            vec![6, 7]
        } else {
            (0..k).collect()
        };

        let t_start = Instant::now();
        for &cpu in &assigned_cpus {
            let b = Arc::clone(&barrier);
            handles.push(thread::spawn(move || run_worker_stream(cpu, b)));
        }

        let mut total_gb_s = 0.0;
        for h in handles {
            total_gb_s += h.join().expect("Worker failed");
        }
        let total_wall = t_start.elapsed().as_secs_f64();

        println!(
            "  k = {:<1} workers: Aggregate Bandwidth = {:>6.2} GB/s (Wall = {:>5.3}s)",
            k, total_gb_s, total_wall
        );
    }
}
