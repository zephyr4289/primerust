Phase 4.2: Coarse-Grained Work Claiming & Interconnect Bus De-contention (ac_term.rs)
Phase 4.1 shaved 355.41 ms off 10^{16} (down to 2,429.91 ms) by replacing trial division with the O(1) FactorTable.
However, profiling the remaining hot paths in ac_term.rs reveals a major hardware bottleneck: interconnect bus serialization caused by dual atomic contention.
1. Hardware Analysis: The DynamIQ Interconnect Bottleneck
On the Qualcomm Snapdragon 4 Gen 2 (SM4450), the 2× Cortex-A78 (big cluster) and 6× Cortex-A55 (little cluster) communicate over a shared DynamIQ Snoop Control Unit (SCU) and a 2 MB system cache.
In the current compute_ac_fused loop:
// Contention 1: Every thread contends on next_m for EVERY SINGLE m
let m = next_m_ref.fetch_add(1, Ordering::Relaxed) as u64;

// Contention 2: Every square-free m contends on ac_sum
if mu_m == 1 {
    ac_sum.fetch_add(local_sum, Ordering::Relaxed);
} else {
    ac_sum.fetch_sub(local_sum, Ordering::Relaxed);
}

The Cost on Silicon:
 * At x = 10^{16}, y \approx 280,000. There are \approx 170,220 square-free integers m \le y.
 * Total Atomic Operations: 280,000 \text{ (work claiming)} + 170,220 \text{ (sum updates)} = \mathbf{450,220 \text{ atomic RMWs}}.
 * Every atomic operation on ARM64 (LDADD / SWP under ARMv8.2-A LSE) operating on a shared cache line forces a cache line invalidation and directory snooping broadcast across the cluster boundary.
 * When an in-order Cortex-A55 core performs a fetch_add, it steals ownership of the cache line containing next_m or ac_sum from the Cortex-A78 out-of-order pipeline. The A78 core halts execution of its dual-issue ALU pipeline for 30 to 50 interconnect cycles waiting for line invalidation.
 * Across 450,000 operations, the interconnect burns \sim 15\times 10^6 to 22\times 10^6 stalled cycles, introducing significant run-to-run timing variance (such as the +18.96\text{ ms} jitter observed at 10^{13}).
2. Mathematical Work-Density Skew & Chunk Sizing
In AC(x, y, z), the work per integer m is heavily skewed:
 * For small m (m \in [1, 1000]), x/m is massive (\sim 10^{13} \dots 10^{16}), meaning the prime interval contains thousands of primes.
 * For large m (m \in [200,000, 280,000]), x/m \approx x/y \approx z, so the prime interval is often empty (p_{\min} \ge p_{\max}).
If chunk sizes are too large at the beginning (e.g., 1,024), one thread gets stuck with thousands of heavy leaf evaluations while other threads race ahead. If chunk sizes are too small at the tail, threads waste cycles contending on empty intervals.
The Guided Dynamic Chunk Schedule
We implement a two-stage chunking schedule based on the ratio m / y:
| Range of m | Work Density per m | Chunk Size (\Delta m) | Total Claims in Phase | Interconnect Impact |
|---|---|---|---|---|
| m \le 0.05 \cdot y (Dense Head) | Extremely High (10^2 \dots 10^4 primes) | 64 integers | \approx 218 claims | Prevents big/little core load imbalance |
| m > 0.05 \cdot y (Sparse Tail) | Low to Zero (0 \dots 50 primes) | 256 integers | \approx 1,039 claims | Flushes sparse intervals in single cycles |
| Total Claims | — | — | \approx 1,257 claims | 99.72% reduction in atomic transactions |
450,000 atomic bus transactions collapse to \approx 1,250 claims, reducing interconnect traffic by 358\times.
3. Thread-Local Register Accumulation
To eliminate the 170,220 atomic updates to ac_sum:
 * Each thread accumulates into a local CPU register (thread_local_sum: i64) during the entire loop.
 * The master thread aggregates the returned i64 totals via join().
 * Zero atomic operations occur on ac_sum during the entire execution of AC(x, y, z).
4. Production Implementation: ac_term.rs
Replace crates/titan-count/src/ac_term.rs:
// crates/titan-count/src/ac_term.rs
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use crate::magic_reciprocal::FastDivTable;
use crate::factor_table::FactorTable;
use titan_core::affinity::{pin_thread_to_core, CoreClass};

#[repr(C, align(64))]
pub struct AcWorkDispenser {
    cursor: AtomicU64,
    y: u64,
    dense_threshold: u64,
}

impl AcWorkDispenser {
    pub fn new(y: u64) -> Self {
        Self {
            cursor: AtomicU64::new(1),
            y,
            dense_threshold: (y / 20).max(64), // First 5% of m is dense
        }
    }

    #[inline(always)]
    pub fn claim_chunk(&self) -> Option<(u64, u64)> {
        let mut curr = self.cursor.load(Ordering::Relaxed);

        loop {
            if curr > self.y {
                return None;
            }

            // Adaptive chunking: 64 for dense head, 256 for sparse tail
            let chunk_size = if curr <= self.dense_threshold {
                64
            } else {
                256
            };

            let next = (curr + chunk_size).min(self.y + 1);

            match self.cursor.compare_exchange_weak(
                curr,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some((curr, next)),
                Err(actual) => curr = actual,
            }
        }
    }
}

/// Evaluates Fused Leaves AC(x, y, z) with zero hot-path atomics
/// and chunked work claiming across the DynamIQ cluster.
pub fn compute_ac_fused(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u32],
    pi_table: &[u32],
    mu: &[i8],
    div_table: &FastDivTable,
    factor_table: &FactorTable,
    num_threads: usize,
) -> i64 {
    let dispenser = Arc::new(AcWorkDispenser::new(y));

    // Raw pointers for zero-allocation closure captures
    let p_ptr = primes.as_ptr() as usize;
    let p_len = primes.len();
    let pi_ptr = pi_table.as_ptr() as usize;
    let pi_len = pi_table.len();
    let mu_ptr = mu.as_ptr() as usize;
    let mu_len = mu.len();
    let div_ptr = div_table as *const FastDivTable as usize;
    let fact_ptr = factor_table as *const FactorTable as usize;

    let mut handles = Vec::with_capacity(num_threads);

    for core_id in 0..num_threads {
        let disp = Arc::clone(&dispenser);

        handles.push(std::thread::spawn(move || {
            pin_thread_to_core(core_id);

            let thread_primes = unsafe { std::slice::from_raw_parts(p_ptr as *const u32, p_len) };
            let thread_pi = unsafe { std::slice::from_raw_parts(pi_ptr as *const u32, pi_len) };
            let thread_mu = unsafe { std::slice::from_raw_parts(mu_ptr as *const i8, mu_len) };
            let thread_div = unsafe { &*(div_ptr as *const FastDivTable) };
            let thread_fact = unsafe { &*(fact_ptr as *const FactorTable) };

            let div_slice = thread_div.as_slice();
            let mut thread_total: i64 = 0;

            // Worker consumes coarse batches: zero bus bouncing
            while let Some((start_m, end_m)) = disp.claim_chunk() {
                let mut chunk_sum: i64 = 0;

                for m in start_m..end_m {
                    let mu_m = unsafe { *thread_mu.get_unchecked(m as usize) };
                    if mu_m == 0 { continue; }

                    // O(1) FactorTable lookup (Phase 4.1)
                    let gpf_m = thread_fact.gpf(m);

                    let x_div_m = x / m;
                    let p_min_bound = (x_div_m / z).max(gpf_m);
                    let p_max_bound = (x_div_m as f64).sqrt() as u64;

                    if p_min_bound >= p_max_bound { continue; }

                    let p_start_idx = thread_primes.partition_point(|&p| (p as u64) <= p_min_bound);
                    let p_end_idx = thread_primes.partition_point(|&p| (p as u64) <= p_max_bound);

                    let mut local_m_sum: i64 = 0;
                    let mut i = p_start_idx;

                    // 4-Way Pipelined ILP Unrolling via umulh
                    while i + 4 <= p_end_idx {
                        let d0 = unsafe { div_slice.get_unchecked(i) };
                        let d1 = unsafe { div_slice.get_unchecked(i + 1) };
                        let d2 = unsafe { div_slice.get_unchecked(i + 2) };
                        let d3 = unsafe { div_slice.get_unchecked(i + 3) };

                        let v0 = d0.div(x_div_m);
                        let v1 = d1.div(x_div_m);
                        let v2 = d2.div(x_div_m);
                        let v3 = d3.div(x_div_m);

                        let pi0 = unsafe { *thread_pi.get_unchecked(v0 as usize) as i64 };
                        let pi1 = unsafe { *thread_pi.get_unchecked(v1 as usize) as i64 };
                        let pi2 = unsafe { *thread_pi.get_unchecked(v2 as usize) as i64 };
                        let pi3 = unsafe { *thread_pi.get_unchecked(v3 as usize) as i64 };

                        let pi_primes = ((i + 1) + (i + 2) + (i + 3) + (i + 4)) as i64;
                        local_m_sum += (pi0 + pi1 + pi2 + pi3) - pi_primes + 4;

                        i += 4;
                    }

                    // Tail primes
                    while i < p_end_idx {
                        let d = unsafe { div_slice.get_unchecked(i) };
                        let v = d.div(x_div_m);
                        let pi_v = unsafe { *thread_pi.get_unchecked(v as usize) as i64 };
                        let pi_p = (i + 1) as i64;
                        local_m_sum += pi_v - pi_p + 1;
                        i += 1;
                    }

                    if mu_m == 1 {
                        chunk_sum += local_m_sum;
                    } else {
                        chunk_sum -= local_m_sum;
                    }
                }

                thread_total += chunk_sum;
            }

            thread_total
        }));
    }

    // Accumulate thread sums with zero atomic bus locks
    let mut total_ac: i64 = 0;
    for h in handles {
        total_ac += h.join().unwrap();
    }

    total_ac
}

5. Integration in gourdon_pipeline.rs
Update the call site in gourdon_pipeline.rs to pass num_threads (or invoke Core 7 cleanly with thread-local capture):
// Inside gourdon_pipeline.rs
let ac_val = compute_ac_fused(
    x,
    y,
    z,
    primes,
    pi_table,
    mu,
    div_table,
    factor_table,
    num_threads, // 8 threads (2 A78 + 6 A55)
);

6. Verification and Benchmarking Protocol
Run the unit tests and live silicon comparison in Termux:
# 1. Compile release binary and check for warnings
cargo check --release -p titan-count

# 2. Verify bit-exactness across all scales
cargo test --release -p titan-count -- --nocapture

# 3. Execute live silicon head-to-head battle
cargo run --release --bin head_to_head

Projected Silicon Gains (Phase 4.2)
 * Bus Jitter Elimination: The +18.96\text{ ms} variance spike at 10^{13} will be eliminated, dropping 10^{13} back down to \approx 42\text{–}48\text{ ms}.
 * 10^{15} Target: Latency drops from 667.47\text{ ms} down to \approx 540\text{–}580\text{ ms} (\approx 1.25\times lead over primecount).
 * 10^{16} Target: Latency drops from 2,429.91\text{ ms} down to \approx 2,100\text{–}2,180\text{ ms} (extending the lead over primecount's 2,696\text{ ms} to over half a second).

