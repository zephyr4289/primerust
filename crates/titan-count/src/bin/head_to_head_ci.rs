//! CI-2 harness: single-shot TierDispatch timing for head-to-head vs primecount.
//!
//! Usage: head_to_head_ci [--threads N] [--trials N] <x1> [x2 ...]
//! Defaults: threads = CpuTopology ncpu, trials = 3 (median reported).
//! Prints `x pi ms` per scale + expected check (pi.toml anchors).
//! primecount side runs separately in CI workflow (same machine, same threads).

use std::time::Instant;
use titan_count::tier_dispatch::TierDispatch;

fn expected_pi(x: u64) -> Option<u64> {
    Some(match x {
        1_000_000 => 78_498,
        10_000_000 => 664_579,
        100_000_000 => 5_761_455,
        1_000_000_000 => 50_847_534,
        10_000_000_000 => 455_052_511,
        100_000_000_000 => 4_118_054_813,
        1_000_000_000_000 => 37_607_912_018,
        10_000_000_000_000 => 346_065_536_839,
        100_000_000_000_000 => 3_204_941_750_802,
        1_000_000_000_000_000 => 29_844_570_422_669,
        10_000_000_000_000_000 => 279_238_341_033_925,
        100_000_000_000_000_000 => 2_623_557_157_654_233,
        1_000_000_000_000_000_000 => 24_739_954_287_740_860,
        _ => return None,
    })
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut threads = titan_core::cpu::CpuTopology::detect().ncpu;
    let mut trials = 3usize;
    let mut xs: Vec<u64> = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--threads" => {
                threads = raw.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(threads);
                i += 2;
            }
            "--trials" => {
                trials = raw.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(trials).max(1);
                i += 2;
            }
            s => {
                if let Ok(x) = s.parse::<u64>() {
                    xs.push(x);
                }
                i += 1;
            }
        }
    }
    if xs.is_empty() {
        xs = vec![1_000_000, 10_000_000, 100_000_000, 1_000_000_000, 10_000_000_000, 100_000_000_000, 1_000_000_000_000];
    }
    let topo = titan_core::cpu::CpuTopology::detect();
    eprintln!("[CI-HARNESS] ncpu={} kind={:?} threads={} trials={}", topo.ncpu, topo.kind, threads, trials);

    for x in xs {
        let mut dts = Vec::with_capacity(trials);
        let mut ans = 0u64;
        for _ in 0..trials {
            let t0 = Instant::now();
            ans = TierDispatch::count(x, threads);
            dts.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let med = median(dts);
        let status = match expected_pi(x) {
            Some(e) if e == ans => "OK",
            Some(e) => {
                eprintln!("[FAIL] x={x} got {ans} expected {e}");
                std::process::exit(1);
            }
            None => "UNPINNED",
        };
        println!("x={x} pi={ans} median_ms={med:.1} status={status}");
    }
}
