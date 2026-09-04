//! Live Head-to-Head Ultra-Scale Benchmark: Titan vs Kim Walisch's primecount 8.1 (10^17 & 10^18)
//!
//! Silicon Platform: Qualcomm Snapdragon 4 Gen 2 (SM4450 | 2x Cortex-A78 + 6x Cortex-A55)
//! Threads: 8 (Heterogeneous Big.LITTLE DynamIQ)

use std::hint::black_box;
use std::time::Instant;
use titan_count::tier_dispatch::TierDispatch;

fn get_primecount_cmd() -> std::process::Command {
    if let Ok(prefix) = std::env::var("PREFIX") {
        let termux_path = format!("{}/bin/primecount", prefix);
        if std::path::Path::new(&termux_path).exists() {
            return std::process::Command::new(termux_path);
        }
    }
    for path in &[
        "/data/data/com.termux/files/usr/bin/primecount",
        "/usr/local/bin/primecount",
        "/usr/bin/primecount",
    ] {
        if std::path::Path::new(path).exists() {
            return std::process::Command::new(path);
        }
    }
    std::process::Command::new("primecount")
}

fn run_primecount(x: u64, threads: usize) -> (u64, f64) {
    let t0 = Instant::now();
    let output = get_primecount_cmd()
        .arg(x.to_string())
        .arg("-t")
        .arg(threads.to_string())
        .output()
        .expect("Failed to execute primecount");
    let elapsed = t0.elapsed().as_secs_f64() * 1000.0; // ms

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result = stdout.trim().lines().last().unwrap_or("0").parse::<u64>().unwrap_or(0);
    (result, elapsed)
}

fn run_ultra_scale(x: u64, expected: u64) {
    println!("\n========================================================");
    println!("  RUNNING ULTRA-SCALE: x = 10^{}", (x as f64).log10().round() as u32);
    println!("  Target π(x) = {}", expected);
    println!("========================================================");

    let autotuner = titan_core::autotuner::EmpiricalAutotuner {
        cost_per_ac_leaf: 28.5,
        cost_per_d_segment: 2750.0,
    };
    let params = autotuner.optimize(x);
    let y = params.y;
    let z = params.z;
    let span = titan_sieve::wheel30::WHEEL_SPAN;
    let segs = if params.x_div_y > z { ((params.x_div_y - z) + span - 1) / span } else { 0 };

    println!("  Autotuned Dynamic Tuning: alpha_y = {:.2}, alpha_z = {:.2}", params.alpha_y, params.alpha_z);
    println!("  Parameters              : y = {}, z = {}, Endpoint x/y = {}", y, z, params.x_div_y);
    println!("  Wheel-30 Segs           : {} segments in D (Span = {} integers)", segs, span);

    // Run Primecount
    println!("  Executing primecount 8.1 (8 threads)...");
    let (pc_res, pc_ms) = run_primecount(x, 8);
    println!("  Primecount Result : {} in {:.2} ms ({:.3} s)", pc_res, pc_ms, pc_ms / 1000.0);

    println!("  Allowing passive heatsink cooldown (30 seconds before Titan)...");
    std::thread::sleep(std::time::Duration::from_secs(30));

    // Run Titan
    println!("  Executing Titan Heterogeneous Engine (8 DynamIQ Cores)...");
    let t0 = Instant::now();
    let titan_res = TierDispatch::count(black_box(x), 8);
    let titan_ms = t0.elapsed().as_secs_f64() * 1000.0;

    println!("  Titan Result      : {} in {:.2} ms ({:.3} s)", titan_res, titan_ms, titan_ms / 1000.0);
    assert_eq!(pc_res, expected, "Primecount failed expected target");
    assert_eq!(titan_res, expected, "Titan failed bit-exact target");

    let ratio = pc_ms / titan_ms;
    println!("  ------------------------------------------------------");
    println!("  Bit-Exact Status  : ✅ 100% BIT-EXACT MATCH");
    println!("  Performance Ratio : {:.2}x FASTER than primecount", ratio);
    println!("========================================================");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_19_only = args.iter().any(|a| a == "1e19" || a == "19");
    let is_18_only = args.iter().any(|a| a == "1e18" || a == "18");
    let is_17_only = args.iter().any(|a| a == "1e17" || a == "17");

    let (run_17, run_18, run_19) = if is_19_only {
        (false, false, true)
    } else if is_18_only {
        (false, true, false)
    } else if is_17_only {
        (true, false, false)
    } else {
        (true, true, false)
    };

    println!("══════════════════════════════════════════════════════════════════════════════════════════════════");
    println!("ULTRA-SCALE BATTLE: TITAN vs KIM WALISCH's PRIMECOUNT 8.1");
    println!("Silicon Platform: Qualcomm Snapdragon 4 Gen 2 (SM4450 | 2x Cortex-A78 + 6x Cortex-A55)");
    println!("Threads: 8 (Heterogeneous Big.LITTLE DynamIQ)");
    println!("══════════════════════════════════════════════════════════════════════════════════════════════════\n");

    if run_17 {
        // Scale 10^17: 2,623,557,157,654,233
        run_ultra_scale(100_000_000_000_000_000, 2_623_557_157_654_233);
        if run_18 || run_19 {
            println!("\nAllowing passive heatsink cooldown (35 seconds)...");
            std::thread::sleep(std::time::Duration::from_secs(35));
        }
    }

    if run_18 {
        // Scale 10^18: 24,739,954,287,740,860
        run_ultra_scale(1_000_000_000_000_000_000, 24_739_954_287_740_860);
        if run_19 {
            println!("\nAllowing passive heatsink cooldown (45 seconds)...");
            std::thread::sleep(std::time::Duration::from_secs(45));
        }
    }

    if run_19 {
        // Scale 10^19: 234,057,667,276,344,607
        run_ultra_scale(10_000_000_000_000_000_000, 234_057_667_276_344_607);
    }
}
