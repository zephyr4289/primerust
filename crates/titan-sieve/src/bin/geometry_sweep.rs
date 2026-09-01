//! Pre-Flight Experiment P0c: L1D Geometry Sweep on Snapdragon 4 Gen 2.
//!
//! Sweeps segment size S in {16K, 32K, 48K, 64K, 96K, 128K} on both
//! Cortex-A78 (big core) and Cortex-A55 (little core) to identify
//! the true L1D cache summit on the new silicon.

use std::time::Instant;
use titan_bench::{pin, snapshot};
use titan_sieve::arena::SieveArena;
use titan_sieve::segment::count_primes_with_arena;

fn benchmark_geometry(cpu: usize, seg_size: usize, limit: u64) -> (f64, u64) {
    let _ = pin::set_affinity(cpu);
    let mut arena = SieveArena::new(limit, seg_size);

    // Warmup
    let _ = count_primes_with_arena(1_000_000_000, seg_size, &mut arena);

    // 3 runs, min time
    let mut min_duration = std::time::Duration::from_secs(1000);
    let mut count = 0u64;

    for _ in 0..3 {
        let t0 = Instant::now();
        count = count_primes_with_arena(limit, seg_size, &mut arena);
        let d = t0.elapsed();
        if d < min_duration {
            min_duration = d;
        }
    }

    let rate = (limit as f64) / min_duration.as_secs_f64();
    (rate, count)
}

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== PRE-FLIGHT EXPERIMENT P0c: L1D GEOMETRY SWEEP ==");

    let limit = 10_000_000_000u64; // 10^10 burst
    let segment_sizes = [
        (16 * 1024, "16 KiB"),
        (32 * 1024, "32 KiB"),
        (48 * 1024, "48 KiB"),
        (64 * 1024, "64 KiB"),
        (96 * 1024, "96 KiB"),
        (128 * 1024, "128 KiB"),
    ];

    println!("\n[1/2] Sweeping Big Core (Cortex-A78 - cpu6) at N=10^10...");
    let mut best_big_rate = 0.0;
    let mut best_big_seg = 0;

    for &(s, label) in &segment_sizes {
        let (rate, cnt) = benchmark_geometry(6, s, limit);
        assert_eq!(cnt, 455_052_511, "Count mismatch at size {}", label);
        let g_rate = rate / 1e9;
        println!("  Cortex-A78 | S = {:>7} : {:>6.3} B/s (wall={:.3}s)", label, g_rate, (limit as f64) / rate);
        if g_rate > best_big_rate {
            best_big_rate = g_rate;
            best_big_seg = s;
        }
    }
    println!("  >> Big Core Peak Summit: {} ({:.3} B/s)", best_big_seg / 1024, best_big_rate);

    println!("\n[2/2] Sweeping Little Core (Cortex-A55 - cpu0) at N=10^10...");
    let mut best_little_rate = 0.0;
    let mut best_little_seg = 0;

    for &(s, label) in &segment_sizes {
        let (rate, cnt) = benchmark_geometry(0, s, limit);
        assert_eq!(cnt, 455_052_511, "Count mismatch at size {}", label);
        let g_rate = rate / 1e9;
        println!("  Cortex-A55 | S = {:>7} : {:>6.3} B/s (wall={:.3}s)", label, g_rate, (limit as f64) / rate);
        if g_rate > best_little_rate {
            best_little_rate = g_rate;
            best_little_seg = s;
        }
    }
    println!("  >> Little Core Peak Summit: {} ({:.3} B/s)", best_little_seg / 1024, best_little_rate);

    let _ = pin::set_full_affinity();
    println!("\n=== GEOMETRY SWEEP COMPLETE ===");
    println!("  Recommended Big Core Segment Size   : {} KiB", best_big_seg / 1024);
    println!("  Recommended Little Core Segment Size: {} KiB", best_little_seg / 1024);
}
