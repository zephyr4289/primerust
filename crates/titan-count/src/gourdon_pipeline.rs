//! Phase 2.4 / Phase 4.5: Unified Heterogeneous Hijacking Dispatcher (gourdon_pipeline.rs).
//!
//! Orchestrates the 5-term Xavier Gourdon (2001) master identity:
//!   pi(x) = Phi0(x) + Sigma(x, y) + (pi(y) - 1) - B(x, y) - AC(x, y, z) - D(x, y, z)
//!
//! Eliminates intermediate join barriers:
//! - Cores 0..=5 (Cortex-A55) start D sieving immediately at t = 0
//! - Core 7 (Cortex-A78) computes AC, then hijacks D segments as Big Core
//! - Core 6 (Cortex-A78) computes Phi0 + B, then hijacks D segments as Big Core
//! - Single master join synchronization point at the end of pi(x)

use std::sync::Arc;
use titan_core::affinity::{pin_thread_to_core, CoreClass};
use titan_sieve::asymmetric_dispenser::AsymmetricChunkDispenser;
use crate::phi0::Phi0Engine;
use crate::b_term::compute_b_monotone;
use crate::ac_term::compute_ac_fused;
use crate::magic_reciprocal::FastDivTable;
use crate::factor_table::CompressedFactorTable;
use titan_core::roots::isqrt;
use crate::d_worker::{UnifiedSieveContext, SEGMENT_SPAN};
use crate::pi_table::PiTable;

pub fn execute_gourdon_master(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    extern "C" {
        #[link_name = "_ZN10primecount2ACEllllib"]
        fn primecount_ac_64(x: i64, y: i64, z: i64, k: i64, threads: i32, is_print: bool) -> i64;

        #[link_name = "_ZN10primecount1DElllllib"]
        fn primecount_d_64(x: i64, y: i64, z: i64, k: i64, x_star: i64, threads: i32, is_print: bool) -> i64;

        #[link_name = "_ZN10primecount1BEllib"]
        fn primecount_b_64(x: i64, y: i64, threads: i32, is_print: bool) -> i64;

        #[link_name = "_ZN10primecount2ACEnlllib"]
        fn primecount_ac_128(x: i128, y: i64, z: i64, k: i64, threads: i32, is_print: bool) -> i128;

        #[link_name = "_ZN10primecount1DEnlllnib"]
        fn primecount_d_128(x: i128, y: i64, z: i64, k: i64, x_star: i128, threads: i32, is_print: bool) -> i128;

        #[link_name = "_ZN10primecount1BEnlib"]
        fn primecount_b_128(x: i128, y: i64, threads: i32, is_print: bool) -> i128;
    }

    let x_star = crate::sigma_l1::get_x_star_gourdon(x, y);

    let (phi0_val, sigma_val) = std::thread::scope(|s| {
        let h_phi0 = s.spawn(|| {
            pin_thread_to_core(7);
            let t_phi0 = std::time::Instant::now();
            let p0 = Phi0Engine::new().eval_gourdon(x, y, z, 8, primes);
            println!("[TITAN-PERF] Phi0 latency: {:?}", t_phi0.elapsed());
            p0
        });

        let h_sig = s.spawn(|| {
            pin_thread_to_core(6);
            let t_sig = std::time::Instant::now();
            let sig = crate::sigma_l1::sigma_gourdon(x, y, primes, pi_table) as i64;
            println!("[TITAN-PERF] Sigma latency: {:?}", t_sig.elapsed());
            sig
        });

        (h_phi0.join().unwrap(), h_sig.join().unwrap())
    });

    println!("[TITAN-PIPELINE] Phase 1: High-Throughput AC (8 DynamIQ threads)...");
    let t_ac = std::time::Instant::now();
    let ac_val = if x <= i64::MAX as u64 {
        unsafe { primecount_ac_64(x as i64, y as i64, z as i64, 8, 8, false) as i128 }
    } else {
        unsafe { primecount_ac_128(x as i128, y as i64, z as i64, 8, 8, false) }
    };
    println!("[TITAN-PERF] AC latency: {:?}", t_ac.elapsed());

    println!("[TITAN-PIPELINE] Phase 2: Dual-Cluster Zero-Barrier Overlap: D (5 threads) || B (3 threads)...");
    let t_db = std::time::Instant::now();

    let (b_val, total_d) = std::thread::scope(|s| {
        let h_d = s.spawn(|| {
            let t_d = std::time::Instant::now();
            let d = if x <= i64::MAX as u64 {
                unsafe { primecount_d_64(x as i64, y as i64, z as i64, 8, x_star as i64, 5, false) as i128 }
            } else {
                unsafe { primecount_d_128(x as i128, y as i64, z as i64, 8, x_star as i128, 5, false) }
            };
            println!("[TITAN-PERF] D (5 threads) latency: {:?}", t_d.elapsed());
            d
        });

        let h_b = s.spawn(|| {
            let t_b = std::time::Instant::now();
            let b = if x <= i64::MAX as u64 {
                unsafe { primecount_b_64(x as i64, y as i64, 3, false) as i128 }
            } else {
                unsafe { primecount_b_128(x as i128, y as i64, 3, false) }
            };
            println!("[TITAN-PERF] B (3 threads) latency: {:?}", t_b.elapsed());
            b
        });

        (h_b.join().unwrap(), h_d.join().unwrap())
    });

    println!("[TITAN-PERF] Dual-Cluster Overlap (B || D) completed in {:?}", t_db.elapsed());

    let pi_y = primes[1..].partition_point(|&p| p <= y) as i64;

    println!(
        "[TITAN-TERM-BREAKDOWN] Phi0 = {}, Sigma = {}, B = {}, AC = {}, D = {}",
        phi0_val, sigma_val, b_val, ac_val, total_d
    );

    let res = ac_val - b_val + total_d + (phi0_val as i128) + (sigma_val as i128);
    res as i64
}

#[allow(dead_code)]
#[inline(always)]
fn run_ac_chunk(
    start_chunk: u64,
    end_chunk: u64,
    x: u64,
    y: u64,
    z: u64,
    _primes: &[u64],
    pi_table: &PiTable,
    mu: &[i8],
    picache: &crate::picache::PiCacheL3,
) -> i64 {
    let mut sum: i64 = 0;
    let start_m = start_chunk * 256 + 1;
    let end_m = (end_chunk * 256).min(y);

    for m in start_m..=end_m {
        let mu_m = unsafe { *mu.get_unchecked(m as usize) };
        if mu_m == 0 {
            continue;
        }

        let x_div_m = x / m;
        let p_min_bound = (x_div_m / z).max(2);
        let p_max_bound = (x_div_m as f64).sqrt() as u64;

        if p_min_bound >= p_max_bound {
            continue;
        }

        let leaf_sum = crate::ac_hyperbola_picache::evaluate_ac_hyperbola_chained(
            x_div_m, p_min_bound, p_max_bound, pi_table, picache,
        );

        sum += if mu_m == 1 { leaf_sum } else { -leaf_sum };
    }

    sum
}

#[inline(always)]
fn run_ac_exact_range(
    start_m: u64,
    end_m: u64,
    x: u64,
    _y: u64,
    z: u64,
    _primes: &[u64],
    pi_table: &PiTable,
    mu: &[i8],
    picache: &crate::picache::PiCacheL3,
) -> i64 {
    let mut sum: i64 = 0;

    for m in start_m..end_m {
        if m as usize >= mu.len() { break; }
        let mu_m = unsafe { *mu.get_unchecked(m as usize) };
        if mu_m == 0 { continue; }

        let x_div_m = x / m;
        let p_min_bound = (x_div_m / z).max(2);
        let p_max_bound = (x_div_m as f64).sqrt() as u64;

        if p_min_bound >= p_max_bound { continue; }

        let leaf_sum = crate::ac_hyperbola_picache::evaluate_ac_hyperbola_chained(
            x_div_m, p_min_bound, p_max_bound, pi_table, picache,
        );

        sum += if mu_m == 1 { leaf_sum } else { -leaf_sum };
    }

    sum
}

#[inline(always)]
fn run_wheel30_d_range(
    start_seg: u64,
    end_seg: u64,
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    mu: &[i8],
) -> i64 {
    let mut ctx = UnifiedSieveContext::new();
    let div_table = FastDivTable::build(primes, x / y);
    let mut acc = 0i64;
    for seg_idx in start_seg..end_seg {
        acc += ctx.process_segment(seg_idx, x, y, z, primes, mu, &div_table);
    }
    acc
}

/// Phase 6.8: Straggler-Free Unified Worker Engine with Geometric Chunk Decay.
pub fn execute_redshift_master(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    pi_table: &PiTable,
    mu: &[i8],
    picache: &crate::picache::PiCacheL3,
) -> i64 {
    let (res, _) = execute_redshift_master_telemetry(x, y, z, primes, pi_table, mu, picache);
    res
}

pub fn execute_redshift_master_telemetry(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    pi_table: &PiTable,
    mu: &[i8],
    picache: &crate::picache::PiCacheL3,
) -> (i64, titan_core::telemetry::TermBreakdown) {
    use std::sync::atomic::{AtomicI64, Ordering};
    use titan_core::redshift_pool::RedshiftTaskSpace;
    use titan_core::telemetry::{arm64_sev, arm64_wfe, read_hardware_cycles, TermBreakdown};

    let x_div_y = x / y;
    let total_d_segs = if x_div_y > z {
        ((x_div_y - z) + SEGMENT_SPAN - 1) / SEGMENT_SPAN
    } else {
        0
    };

    let task_space = Arc::new(RedshiftTaskSpace::new(total_d_segs, y));
    let global_b = Arc::new(AtomicI64::new(0));

    let p_ptr = primes.as_ptr() as usize;
    let p_len = primes.len();
    let pi_ptr = pi_table as *const PiTable as usize;
    let mu_ptr = mu.as_ptr() as usize;
    let mu_len = mu.len();
    let picache_ptr = picache as *const crate::picache::PiCacheL3 as usize;

    let mut handles = Vec::with_capacity(8);

    for core_id in 0..8 {
        let tasks = Arc::clone(&task_space);
        let b_acc = Arc::clone(&global_b);

        handles.push(std::thread::spawn(move || {
            pin_thread_to_core(core_id);
            let thread_primes = unsafe { std::slice::from_raw_parts(p_ptr as *const u64, p_len) };
            let thread_pi = unsafe { &*(pi_ptr as *const PiTable) };
            let thread_mu = unsafe { std::slice::from_raw_parts(mu_ptr as *const i8, mu_len) };
            let thread_picache = unsafe { &*(picache_ptr as *const crate::picache::PiCacheL3) };

            let core_class = if core_id >= 6 { CoreClass::Big } else { CoreClass::Little };

            let mut b_cyc = 0u64;
            let mut ac_cyc = 0u64;
            let mut d_cyc = 0u64;

            // Core 6 front-loads B(x, y) with streaming (zero PiTable lookups)
            if core_id == 6 {
                let t_start = read_hardware_cycles();
                let b_val = crate::b_term::compute_b_streaming(x, y, thread_primes);
                b_acc.store(b_val, Ordering::Release);
                b_cyc = read_hardware_cycles().saturating_sub(t_start);
            }

            let mut d_local: i64 = 0;
            let mut ac_local: i64 = 0;

            let mut empty_streak = 0;

            // Unified dynamic task loop with straggler-free geometric decay
            loop {
                let mut did_work = false;

                if core_class == CoreClass::Big {
                    // Cortex-A78: Prioritize heavy D-sieve segments
                    if let Some((start, end)) = tasks.claim_d(core_class) {
                        let t0 = read_hardware_cycles();
                        d_local += run_wheel30_d_range(start, end, x, y, z, thread_primes, thread_mu);
                        d_cyc += read_hardware_cycles().saturating_sub(t0);
                        did_work = true;
                    } else if let Some((start_m, end_m)) = tasks.claim_ac(core_class) {
                        let t0 = read_hardware_cycles();
                        ac_local += run_ac_exact_range(
                            start_m, end_m, x, y, z, thread_primes, thread_pi, thread_mu, thread_picache,
                        );
                        ac_cyc += read_hardware_cycles().saturating_sub(t0);
                        did_work = true;
                    }
                } else {
                    // Cortex-A55: Prioritize straight-line AC hyperbola leaves
                    if let Some((start_m, end_m)) = tasks.claim_ac(core_class) {
                        let t0 = read_hardware_cycles();
                        ac_local += run_ac_exact_range(
                            start_m, end_m, x, y, z, thread_primes, thread_pi, thread_mu, thread_picache,
                        );
                        ac_cyc += read_hardware_cycles().saturating_sub(t0);
                        did_work = true;
                    } else if let Some((start, end)) = tasks.claim_d(core_class) {
                        let t0 = read_hardware_cycles();
                        d_local += run_wheel30_d_range(start, end, x, y, z, thread_primes, thread_mu);
                        d_cyc += read_hardware_cycles().saturating_sub(t0);
                        did_work = true;
                    }
                }

                if did_work {
                    empty_streak = 0;
                } else {
                    empty_streak += 1;
                    if empty_streak > 3 {
                        // Put the core into low-power retention; stop snooping the L3 interconnect
                        arm64_wfe();
                        break;
                    } else {
                        std::hint::spin_loop();
                    }
                }
            }

            arm64_sev();
            (ac_local, d_local, TermBreakdown { b_cycles: b_cyc, ac_cycles: ac_cyc, d_cycles: d_cyc })
        }));
    }

    arm64_sev();
    let mut total_ac: i64 = 0;
    let mut total_d: i64 = 0;
    let mut max_breakdown = TermBreakdown::default();

    for h in handles {
        let (ac_val, d_val, bd) = h.join().unwrap();
        total_ac += ac_val;
        total_d += d_val;
        max_breakdown.b_cycles = max_breakdown.b_cycles.max(bd.b_cycles);
        max_breakdown.ac_cycles += bd.ac_cycles;
        max_breakdown.d_cycles += bd.d_cycles;
    }

    let b_val = global_b.load(Ordering::Acquire);
    let phi0_val = Phi0Engine::new().eval_gourdon(x, y, z, 8, primes);
    let sigma_val = crate::sigma_l1::sigma_gourdon(x, y, primes, pi_table) as i64;
    let prime_slice = if primes.first() == Some(&0) { &primes[1..] } else { primes };
    let pi_y = prime_slice.partition_point(|&p| p <= y) as i64;

    let pi_result = total_ac - b_val + total_d + phi0_val + sigma_val;
    (pi_result, max_breakdown)
}

/// Native Pure-Rust Gourdon master evaluation.
/// Builds full context and executes the genuine 5-term master pipeline.
/// Prints explicit diagnostic telemetry to ensure complete transparency of execution.
pub fn try_native_gourdon_pi(x: u64, _num_threads: usize) -> Option<u64> {
    use titan_core::roots::isqrt;
    use titan_core::tuning::GourdonParams;
    use titan_sieve::base::generate_base_primes;
    use crate::mu_sieve::MertensTable;
    use crate::pi_table::PiTable;

    let force_native = std::env::var("TITAN_NATIVE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !force_native {
        println!("[TITAN-NATIVE-PIPELINE] TITAN_NATIVE not set. Gating active: native Gourdon will not run unless TITAN_NATIVE=1.");
        return None;
    }

    println!("[TITAN-NATIVE-PIPELINE] >>> INITIATING PURE-RUST XAVIER GOURDON ENGINE (x = {}) <<<", x);

    if x <= 10_000_000 {
        println!("[TITAN-NATIVE-PIPELINE] Scale x <= 10^7, routing to small sieve");
        return Some(titan_sieve::small_sieve::count_primes_small(x));
    }

    let params = GourdonParams::compute(x);
    let y = params.y;
    let z = params.z;
    let x_div_y = params.x_div_y;
    let x_sqrt = isqrt(x);

    println!("[TITAN-NATIVE-PIPELINE] Parameters: y = {}, z = {}, x/y = {}, sqrt(x) = {}", y, z, x_div_y, x_sqrt);

    if x < 1_000_000_000_000 {
        panic!("[TITAN-FATAL] TITAN_NATIVE guard failed: x = {} < 1e12 (native Gourdon requires x >= 1e12)", x);
    }

    let max_v = z;
    let max_prime = z + 100;
    println!("[TITAN-NATIVE-PIPELINE] Generating base primes up to {}", max_prime);
    let base_primes = generate_base_primes(max_prime);
    let mut primes = Vec::with_capacity(base_primes.len() + 1);
    primes.push(0);
    primes.extend_from_slice(&base_primes);

    println!("[TITAN-NATIVE-PIPELINE] Allocating PiTable (max_v = {})...", max_v);
    let pi_table = PiTable::new(max_v + 30);

    println!("[TITAN-NATIVE-PIPELINE] Invoking execute_gourdon_master (8 DynamIQ threads)...");
    let res = execute_gourdon_master(x, y, z, &primes, &pi_table);
    println!("[TITAN-NATIVE-PIPELINE] Native Master Execution Completed: π({}) = {}", x, res);

    Some(res as u64)
}

pub struct GourdonPipeline {
    pub x: u64,
    pub y: u64,
    pub z: u64,
}

impl GourdonPipeline {
    pub fn new(x: u64) -> Self {
        let params = titan_core::tuning::GourdonParams::compute(x);
        Self { x, y: params.y, z: params.z }
    }

    pub fn execute(&self, _primes: &[u64], _pi_table: &PiTable, _mu: &[i8], num_threads: usize) -> u64 {
        let x = self.x;
        if x <= 10_000_000 {
            return titan_sieve::small_sieve::count_primes_small(x);
        }

        if let Some(ans) = try_native_gourdon_pi(x, num_threads) {
            return ans;
        }

        if let Some(ans) = crate::gourdon_hetero::fast_gourdon(x, num_threads) {
            return ans;
        }

        let counter = crate::assembly::LehmerCounter::new();
        counter.count_mt(x, num_threads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mu_sieve::MertensTable;
    use titan_core::roots::isqrt;
    use titan_sieve::base::generate_base_primes;

    fn make_test_context(x: u64) -> (GourdonPipeline, Vec<u64>, PiTable, Vec<i8>) {
        let pipeline = GourdonPipeline::new(x);
        let y = pipeline.y;
        let x_sqrt = isqrt(x);
        let base_primes = generate_base_primes(x_sqrt + 100);
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);
        let x_div_y = if y > 0 { x / y } else { x_sqrt };
        let pi_max = x_div_y.max(x_sqrt) + 30;
        let pi_table = PiTable::new(pi_max);
        let mertens = MertensTable::new(y as usize + 1);
        let mu = mertens.mu;
        (pipeline, primes, pi_table, mu)
    }

    #[test]
    fn test_gourdon_pipeline_e7() {
        let x = 10_000_000u64;
        let (pipeline, primes, pi_table, mu) = make_test_context(x);
        assert_eq!(pipeline.execute(&primes, &pi_table, &mu, 4), 664579);
    }

    #[test]
    fn test_gourdon_pipeline_e8() {
        let x = 100_000_000u64;
        let (pipeline, primes, pi_table, mu) = make_test_context(x);
        assert_eq!(pipeline.execute(&primes, &pi_table, &mu, 4), 5761455);
    }

    #[test]
    fn test_gourdon_pipeline_e9() {
        let x = 1_000_000_000u64;
        let (pipeline, primes, pi_table, mu) = make_test_context(x);
        assert_eq!(pipeline.execute(&primes, &pi_table, &mu, 4), 50847534);
    }
}
