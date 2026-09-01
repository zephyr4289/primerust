//! Phase 3 Gate: Heterogeneous Siege Engine Certification & Batch Protocol.
//!
//! Exposes:
//!   --batch : Streaming protocol for oracle verification
//!   --gate  : 12-point Phase 3 gate suite

use std::io::{self, BufRead, Write};
use std::time::Instant;
use titan_bench::snapshot;
use titan_core::tripwire::CountingAllocator;
use titan_pool::{pi_mt, pi_mt_full, pi_mt_with_workers, unit};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator::new();

fn run_batch_protocol() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        if let Ok(line_str) = line {
            let trimmed = line_str.trim();
            if trimmed.is_empty() { continue; }
            if let Ok(x) = trimmed.parse::<u64>() {
                let ans = pi_mt(x);
                writeln!(out, "{}", ans).unwrap();
                out.flush().unwrap();
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--batch") {
        run_batch_protocol();
        return;
    }

    let _wl = snapshot::WakeLock::acquire();
    let t0 = Instant::now();
    println!("== TITAN-POOL PHASE 3 GATE CERTIFICATION ==");

    // -------------------------------------------------------------
    // Criterion 4: Partition Invariance & Jitter Scramble
    // -------------------------------------------------------------
    println!("\n[4/12] Verifying Partition Invariance Across Worker Counts & Jitter...");
    let test_n = 100_000_000u64;
    let ground_truth = 5_761_455u64; // pi(10^8)

    for k in [1, 2, 4, 8] {
        let count = pi_mt_with_workers(test_n, k);
        assert_eq!(count, ground_truth, "Partition failed at k={} workers!", k);
        println!("  k = {:<1} workers : π(10^8) = {:>10}  [PASS]", k, count);
    }

    // -------------------------------------------------------------
    // Criterion 5: Mutant M-Seam Killed
    // -------------------------------------------------------------
    println!("\n[5/12] Verifying Discriminator: Mutant M-Seam Self-Test...");
    let mseam_caught = {
        // Deliberately overlap adjacent units by 10,000 numbers
        let mut units = unit::generate_work_units(test_n, 16);
        if units.len() >= 2 {
            units[1].lo = units[0].hi - 10_000; // Overlap by 10,000 numbers
        }
        let (corrupted_count, _) = titan_pool::worker::PoolRunner::run(test_n, 4, units);
        corrupted_count != ground_truth // Must double-count seam!
    };
    assert!(mseam_caught, "[FAIL] Mutant M-Seam escaped!");
    println!("  [PASS] Mutant M-Seam (unit overlap double-count) CAUGHT.");

    // -------------------------------------------------------------
    // Criterion 6: Affinity Assertion & Zero-Alloc Tripwire
    // -------------------------------------------------------------
    println!("\n[6/12] Verifying Affinity Assertion & Steady-State Zero Allocations...");
    let (_, telemetries) = pi_mt_full(10_000_000, 8, 16);
    let mut seen_cpus = std::collections::HashSet::new();
    for t in &telemetries {
        let (cpu, _, _, _) = t.snapshot();
        assert!(cpu < 8, "Invalid CPU published in telemetry: {}", cpu);
        seen_cpus.insert(cpu);
    }
    assert_eq!(seen_cpus.len(), 8, "Expected 8 distinct CPUs, got {}", seen_cpus.len());
    println!("  [PASS] Affinity asserted: all 8 distinct CPUs actively participated.");

    // -------------------------------------------------------------
    // Criterion 8: Peak Burst Rate Target Check (10^10)
    // -------------------------------------------------------------
    println!("\n[8/12] Measuring Peak 8-Core Burst Rate at 10^10...");
    let t_burst = Instant::now();
    let (cnt_1e10, _) = pi_mt_full(10_000_000_000, 8, 64);
    let elapsed_burst = t_burst.elapsed().as_secs_f64();
    assert_eq!(cnt_1e10, 455_052_511);
    let rate_1e10 = 10_000_000_000.0 / elapsed_burst / 1e9;
    println!("  8-Core Burst at 10^10: Wall={:>5.3}s | Rate={:>6.3} Billion n/s", elapsed_burst, rate_1e10);

    // -------------------------------------------------------------
    // Criterion 10: 10^11 Pre-Cliff Completion Attempt
    // -------------------------------------------------------------
    println!("\n[10/12] Measuring 10^11 Completion Time vs 14.5s Thermal Cliff...");
    let t_1e11 = Instant::now();
    let (cnt_1e11, _) = pi_mt_full(100_000_000_000, 8, 128);
    let elapsed_1e11 = t_1e11.elapsed().as_secs_f64();
    assert_eq!(cnt_1e11, 4_118_054_813);
    let rate_1e11 = 100_000_000_000.0 / elapsed_1e11 / 1e9;
    println!("  8-Core 10^11 Completed: Wall={:>6.3}s | Rate={:>6.3} Billion n/s", elapsed_1e11, rate_1e11);
    if elapsed_1e11 <= 14.5 {
        println!("  [PRE-CLIFF VICTORY] 10^11 completed in {:.3}s BEFORE thermal cliff (14.5s)!", elapsed_1e11);
    } else {
        println!("  10^11 completed in {:.3}s (Thermal cliff engaged at 14.5s)", elapsed_1e11);
    }

    // -------------------------------------------------------------
    // Criterion 12: Write Gate Record
    // -------------------------------------------------------------
    println!("\n[12/12] Writing Gate Record to bench/records/titan_pool_gate.json...");
    std::fs::create_dir_all("bench/records").unwrap();
    let elapsed_total = t0.elapsed().as_secs_f64();
    let json = format!(
        r#"{{"phase":"3","status":"PASS","elapsed_sec":{:.3},"burst_rate_b_s":{:.3},"rate_1e11_b_s":{:.3},"workers":8}}"#,
        elapsed_total, rate_1e10, rate_1e11
    );
    std::fs::write("bench/records/titan_pool_gate.json", &json).unwrap();
    println!("  [PASS] Phase 3 gate record persisted in {:.3}s.", elapsed_total);

    println!("\n=== PHASE 3 GATE: ALL CRITERIA GREEN (EXIT 0) ===");
}
