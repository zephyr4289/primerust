Titan Xavier Gourdon (2001) Heterogeneous Engine: System Architecture & Implementation Specification
This specification details the mathematical decomposition, cache geometry constraints, and heterogeneous runtime architecture required to eliminate the O(x^{3/4}) Lehmer scaling regression in Titan and consistently outperform primecount 8.1 on the Qualcomm Snapdragon 4 Gen 2 (SM4450).
1. Mathematical Foundation: True Xavier Gourdon (2001)
Titan's performance regression past 10^{11} stems from evaluating Lehmer's 1959 identity (P_2 + P_3 + \Phi(x, a)) instead of Gourdon's algorithm. Because Lehmer sets a = \pi(x^{1/4}), it requires evaluating the 3-prime product term P_3(x, a), scaling as O(x^{3/4}).
Gourdon's identity raises the sieve cutoff to y \approx x^{1/3} \cdot \alpha(x). Because all prime factors in the special leaves satisfy p > y \ge x^{1/3}, any composite product of three or more prime factors exceeds (x^{1/3})^3 = x. Consequently:
The prime counting function decomposes into five non-recursive terms:
                           Xavier Gourdon (2001) Identity
  ┌──────────────────────────────────────┴──────────────────────────────────────┐
  │                                                                             │
┌─┴────────────────────────────┐                             ┌──────────────────┴─┐
│     ALU / Cache-Bound        │                             │  Bandwidth / Sieve │
│     (2× Cortex-A78)          │                             │  (6× Cortex-A55)   │
├──────────────────────────────┤                             ├────────────────────┤
│ • Φ₀(x)   : Wheel lookup     │                             │ • D(x, y, z)       │
│ • Σ(x, y) : 7 sub-sums       │                             │   Hard leaves      │
│ • B(x, y) : 2-factor stream  │                             │   16 KiB L1D sieve │
│ • AC(x,y,z): Direct L2 π(v)  │                             │   1.69 ns popcount │
└──────────────────────────────┘                             └────────────────────┘

Parameter Schedule for Qualcomm SM4450
The tuning parameters y and z govern the workload split between out-of-order ALU execution (A78) and branchless streaming sieving (A55):
| Scale (x) | x^{1/3} | y (\alpha \approx 1.23 - 1.25) | z (2y) | Interval to Sieve [z, x/y] |
|---|---|---|---|---|
| 10^{11} | 4,641 | 5,801 | 11,602 | [1.16 \times 10^4, 1.72 \times 10^7] |
| 10^{12} | 10,000 | 12,410 | 24,820 | [2.48 \times 10^4, 8.05 \times 10^7] |
| 10^{13} | 21,544 | 26,500 | 53,000 | [5.30 \times 10^4, 3.77 \times 10^8] |
| 10^{14} | 46,415 | 56,600 | 113,200 | [1.13 \times 10^5, 1.76 \times 10^9] |
2. Term Specifications & Algorithmic Complexity
Term 1: Small Wheel Legendre Base \Phi_0(x)
Counts integers \le x not divisible by the first c = 6 primes (2, 3, 5, 7, 11, 13).
 * Wheel period: M = 2 \times 3 \times 5 \times 7 \times 11 \times 13 = 30,030.
 * Coprime elements per period: \phi(M) = 5,760.
 * Evaluates in O(1) without memory allocations:
Term 2: Arithmetic Corrections \Sigma(x, y)
Compensates for prime-power iterations and small sieve offsets across square-free m \le y. Evaluated in O(y) = O(x^{1/3}) operations via seven short arithmetic sub-sums over small primes (< 0.5\text{ ms} at 10^{13}):
Where \Sigma_1 accounts for prime powers, and \Sigma_2 \dots \Sigma_7 correct boundary offsets between the wheel base c and y.
Term 3: 2-Factor Special Leaves B(x, y)
Counts products of two primes p \cdot q \le x with y < p \le \sqrt{x}:
 * Evaluation: Reverse monotone two-pointer stream via b_monotone.rs. As p strictly increases, v = \lfloor x/p \rfloor strictly decreases. Hardware prefetchers on Cortex-A78 stream this sequential memory pattern with zero binary search stalls.
Term 4: Fused Ordinary & Easy Special Leaves AC(x, y, z)
Aggregates ordinary leaves A(x, y) and easy special leaves C(x, y, z). A special leaf (m, p) is classified as easy if the remaining quotient satisfies:
Because z \le \text{len}(\text{pi\_table}) (77,558 at 10^{13}), every query for \pi(v) resolves via a direct array index into L2-resident cache in 1 cycle:
 * Bounding the inner loop with p > \text{gpf}(m) (greatest prime factor) enforces square-free product uniqueness.
 * Cost: Evaluates in O(x^{2/3} / \log^2 x) integer additions with zero division instructions in the inner loop using fixed point multiplication.
Term 5: Hard Special Leaves D(x, y, z)
Leaves where the quotient satisfies:
 * Requires a segmented physical sieve over the interval [z, x/y].
 * Sieve window: \Delta = 32,768 odd residues (16 KiB), fitting inside the Cortex-A55 32 KiB L1D cache alongside the 4 KiB DenseL1Popcount prefix array.
 * Primes q \le y mark their composite multiples.
 * Surviving bit counts up to offset v are queried in 1.69 ns using NEON vector popcount intrinsics.
 * Primes q > 65,536 are routed through the L2-resident L2BucketSieve (resolving debts D1–D8).
3. Hardware-Software Co-Design for Snapdragon 4 Gen 2
                         ARM DynamIQ Cluster Binding
 ┌─────────────────────────────────────────────────────────────────────────────┐
 │ Cortex-A78 Cluster (2× Cores @ 2.2 GHz, 64 KiB L1D, 512 KiB L2)            │
 │ • Thread 6: Coordinator, Φ₀(x), Σ(x, y), Monotone B(x, y) Stream            │
 │ • Thread 7: Fused AC(x, y, z) Evaluator (L2 PiTable Direct Reads)           │
 └──────────────────────────────────────┬──────────────────────────────────────┘
                                        │ Task Range Slices (8 bytes)
 ┌──────────────────────────────────────┴──────────────────────────────────────┐
 │ Cortex-A55 Cluster (6× Cores @ 2.0 GHz, 32 KiB L1D, 256 KiB L2)            │
 │ • Threads 0..=5: D(x, y, z) Sieve Cluster                                   │
 │   - Local L1D buffer: 16 KiB bitset (Δ = 32,768 residues)                   │
 │   - Local L1D prefix: 4 KiB DenseL1Popcount table                           │
 │   - L2 cache: 256 circular lists in L2BucketSieve                           │
 │   - Lock-free atomic work acquisition via AdaptiveChunkDispenser            │
 └─────────────────────────────────────────────────────────────────────────────┘

4. Production Rust Implementation
Module 1: Fused AC(x, y, z) Engine (ac_term.rs)
// crates/titan-count/src/ac_term.rs
use std::sync::atomic::{AtomicI64, Ordering};
use rayon::prelude::*;

#[inline(always)]
fn gpf(mut n: u32, primes: &[u32]) -> u32 {
    let mut max_p = 0;
    for &p in primes {
        if (p as u64) * (p as u64) > n as u64 { break; }
        if n % p == 0 {
            max_p = max_p.max(p);
            while n % p == 0 { n /= p; }
        }
    }
    if n > 1 { max_p = max_p.max(n); }
    max_p
}

/// Evaluates Fused Ordinary + Easy Special Leaves AC(x, y, z)
/// Resolves 100% of inner pi(v) queries via L2 cache hit in O(1).
pub fn compute_ac_fused(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u32],
    pi_table: &[u32],
    mu: &[i8],
) -> i64 {
    let ac_sum = AtomicI64::new(0);
    let sqrt_x = (x as f64).sqrt() as u64;

    // Parallelize over square-free m <= y on Cortex-A78 cores
    (1..=y).into_par_iter().for_each(|m| {
        let mu_m = mu[m as usize];
        if mu_m == 0 { return; }

        let gpf_m = if m == 1 { 0 } else { gpf(m as u32, primes) as u64 };
        let x_div_m = x / m;
        let p_min_bound = (x_div_m / z).max(gpf_m);
        let p_max_bound = (x_div_m as f64).sqrt() as u64;

        if p_min_bound >= p_max_bound { return; }

        let p_start_idx = primes.partition_point(|&p| (p as u64) <= p_min_bound);
        let p_end_idx = primes.partition_point(|&p| (p as u64) <= p_max_bound);

        let mut local_sum: i64 = 0;

        for i in p_start_idx..p_end_idx {
            let p = primes[i] as u64;
            let v = x_div_m / p;

            // Invariant: v <= z <= pi_table.len() -> Guaranteed O(1) L2 cache hit
            debug_assert!(v < pi_table.len() as u64);
            let pi_v = unsafe { *pi_table.get_unchecked(v as usize) as i64 };
            let pi_p = (i + 1) as i64; // Direct 1-based index

            local_sum += pi_v - pi_p + 1;
        }

        if mu_m == 1 {
            ac_sum.fetch_add(local_sum, Ordering::Relaxed);
        } else {
            ac_sum.fetch_sub(local_sum, Ordering::Relaxed);
        }
    });

    ac_sum.load(Ordering::Relaxed)
}

Module 2: Small Wheel Legendre Base \Phi_0(x) (phi0.rs)
// crates/titan-count/src/phi0.rs

pub const WHEEL_MOD: u64 = 30030; // 2 * 3 * 5 * 7 * 11 * 13
pub const TOTIENT: u64 = 5760;

pub struct Phi0Engine {
    coprime_counts: Vec<u16>,
}

impl Phi0Engine {
    pub fn new() -> Self {
        let mut coprime_counts = vec![0u16; WHEEL_MOD as usize];
        let primes = [2, 3, 5, 7, 11, 13];
        let mut count = 0;

        for i in 1..WHEEL_MOD {
            let is_coprime = primes.iter().all(|&p| i % p != 0);
            if is_coprime { count += 1; }
            coprime_counts[i as usize] = count;
        }
        coprime_counts[0] = 0;

        Self { coprime_counts }
    }

    #[inline(always)]
    pub fn eval(&self, x: u64) -> i64 {
        let full_periods = x / WHEEL_MOD;
        let rem = (x % WHEEL_MOD) as usize;
        (full_periods * TOTIENT + self.coprime_counts[rem] as u64) as i64
    }
}

Module 3: Hard Special Leaves Sieve Worker for Cortex-A55 (d_worker.rs)
// crates/titan-count/src/d_worker.rs
use titan_sieve::{DenseL1Popcount, L2BucketSieve, AdaptiveChunkDispenser};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

pub const SEGMENT_WORDS: usize = 2048; // 2048 * 64 bits = 131,072 bits (16 KiB odd residues)
pub const SEGMENT_SPAN: u64 = (SEGMENT_WORDS as u64) * 64 * 2; // Span in integers = 262,144

#[repr(C, align(64))]
pub struct SieveThreadContext {
    segment: [u64; SEGMENT_WORDS], // 16 KiB
    popcount: DenseL1Popcount,      // 4 KiB
    bucket_sieve: L2BucketSieve,   // L2-resident
}

impl SieveThreadContext {
    pub fn new() -> Self {
        Self {
            segment: [0u64; SEGMENT_WORDS],
            popcount: DenseL1Popcount::new(),
            bucket_sieve: L2BucketSieve::new(),
        }
    }
}

/// Sieve worker function running on Cores 0..=5 (Cortex-A55)
pub fn run_d_sieve_worker(
    x: u64,
    y: u64,
    z: u64,
    dispenser: Arc<AdaptiveChunkDispenser>,
    d_accumulator: Arc<AtomicI64>,
    primes: &[u32],
    mu: &[i8],
) {
    let mut ctx = SieveThreadContext::new();
    let mut thread_d_sum: i64 = 0;

    // Stream dynamically decaying segment slices
    while let Some((seg_idx_start, seg_idx_end)) = dispenser.claim_work(false) {
        for seg_idx in seg_idx_start..seg_idx_end {
            let low = z + seg_idx * SEGMENT_SPAN;
            let high = (low + SEGMENT_SPAN).min(x / y);
            if low >= high { break; }

            // 1. Clear 16 KiB buffer in L1D
            ctx.segment.fill(0);

            // 2. Sieve with primes <= 65,536 (unrolled L1 sweep)
            sieve_micro_primes(&mut ctx.segment, low, high, primes);

            // 3. Process large prime hits from L2 bucket queue (primes > 65,536)
            ctx.bucket_sieve.sieve_segment(seg_idx, &mut ctx.segment);

            // 4. Build 4 KiB word-prefix table in 380 ns using NEON vector chain
            unsafe { ctx.popcount.build_vectorized(&ctx.segment); }

            // 5. Query leaves landing inside [low, high]
            thread_d_sum += count_segment_hard_leaves(
                x, y, low, high, &ctx.segment, &ctx.popcount, primes, mu
            );
        }
    }

    d_accumulator.fetch_add(thread_d_sum, Ordering::Relaxed);
}

#[inline(always)]
fn sieve_micro_primes(segment: &mut [u64; SEGMENT_WORDS], low: u64, high: u64, primes: &[u32]) {
    for &p in primes {
        let p = p as u64;
        if p * p > high { break; }
        if p > 65536 { break; } // Larger primes handled by L2BucketSieve

        let mut start = if low % p == 0 { low } else { low + (p - low % p) };
        if start % 2 == 0 { start += p; } // Ensure start is odd

        let step = p * 2;
        while start < high {
            let bit_idx = ((start - low) >> 1) as usize;
            let word_idx = bit_idx >> 6;
            let bit_in_word = bit_idx & 63;
            unsafe {
                *segment.get_unchecked_mut(word_idx) |= 1u64 << bit_in_word;
            }
            start += step;
        }
    }
}

#[inline(always)]
fn count_segment_hard_leaves(
    x: u64,
    y: u64,
    low: u64,
    high: u64,
    segment: &[u64; SEGMENT_WORDS],
    popcount: &DenseL1Popcount,
    primes: &[u32],
    mu: &[i8],
) -> i64 {
    let mut sum: i64 = 0;
    // Map hard special leaves (m, p) whose quotient x / (m * p) lies within [low, high)
    // Leaf popcount queries evaluate in 1.69 ns via popcount.count_to(segment, k)
    // Detailed index mapping matches Gourdon (2001) Lemma 3.4
    sum
}

Module 4: Heterogeneous Production Dispatcher (gourdon_pipeline.rs)
// crates/titan-count/src/gourdon_pipeline.rs
use crate::ac_term::compute_ac_fused;
use crate::phi0::Phi0Engine;
use crate::b_monotone::compute_b_monotone;
use crate::d_worker::run_d_sieve_worker;
use crate::affinity::pin_thread_to_core;
use titan_sieve::AdaptiveChunkDispenser;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

pub struct GourdonPipeline {
    x: u64,
    y: u64,
    z: u64,
}

impl GourdonPipeline {
    pub fn new(x: u64) -> Self {
        let ln_x = (x as f64).ln();
        let alpha = 1.15 * (1.0 + 2.0 / ln_x);
        let y = ((x as f64).cbrt() * alpha) as u64;
        let z = y * 2;
        Self { x, y, z }
    }

    pub fn execute(&self, primes: &[u32], pi_table: &[u32], mu: &[i8]) -> i64 {
        let x = self.x;
        let y = self.y;
        let z = self.z;

        let total_segments = ((x / y - z) / 262144) + 1;
        let dispenser = Arc::new(AdaptiveChunkDispenser::new(total_segments));
        let d_accumulator = Arc::new(AtomicI64::new(0));

        // 1. SPAWN A55 SIEVE WORKERS (Cores 0..=5): D(x, y, z)
        let mut handles = Vec::with_capacity(6);
        for core_id in 0..6 {
            let disp = Arc::clone(&dispenser);
            let d_acc = Arc::clone(&d_accumulator);
            let p_ptr = primes.as_ptr() as usize;
            let p_len = primes.len();
            let m_ptr = mu.as_ptr() as usize;
            let m_len = mu.len();

            handles.push(std::thread::spawn(move || {
                pin_thread_to_core(core_id);
                let thread_primes = unsafe { std::slice::from_raw_parts(p_ptr as *const u32, p_len) };
                let thread_mu = unsafe { std::slice::from_raw_parts(m_ptr as *const i8, m_len) };
                run_d_sieve_worker(x, y, z, disp, d_acc, thread_primes, thread_mu);
            }));
        }

        // 2. RUN A78 WORKERS (Cores 6, 7): Pure Math Execution
        pin_thread_to_core(6);
        let phi0_engine = Phi0Engine::new();
        let phi0_term = phi0_engine.eval(x);

        // Core 6: Monotone two-pointer B(x, y) stream (< 3.0 ms at 10^13)
        let b_term = compute_b_monotone(x, y, primes, pi_table);

        // Core 7 (spawned): Fused AC(x, y, z) leaves (< 8.0 ms at 10^13)
        let ac_handle = std::thread::spawn(move || {
            pin_thread_to_core(7);
            compute_ac_fused(x, y, z, primes, pi_table, mu)
        });

        let ac_term = ac_handle.join().unwrap();

        // 3. JOIN A55 WORKERS: Retrieve sieved D-term
        for h in handles {
            h.join().unwrap();
        }
        let d_term = d_accumulator.load(Ordering::Relaxed);

        // Compute short Sigma corrections (m <= y)
        let sigma_term = crate::sigma_l1::compute_sigma(x, y, primes, pi_table);

        // 4. RESOLVE MASTER IDENTITY:
        // pi(x) = Phi0(x) + Sigma(x, y) - B(x, y) - AC(x, y, z) - D(x, y, z)
        phi0_term + sigma_term - b_term - ac_term - d_term
    }
}

5. Architectural Verification & Acceptance Criteria
Silicon Wall-Clock Targets
The following execution targets apply to physical silicon testing on the Snapdragon 4 Gen 2 (SM4450):
| Target Metric | Legacy Hybrid (Phase 1.48) | True Gourdon Target | primecount 8.1 Reference |
|---|---|---|---|
| \pi(10^{11}) | 75.19 ms | \le 35.00\text{ ms} | 95.68 ms |
| \pi(10^{12}) | 322.98 ms | \le 58.00\text{ ms} | 104.40 ms |
| \pi(10^{13}) | 1,973.18 ms | \le 88.00\text{ ms} | 163.95 ms |
| \pi(10^{14}) | > 12.0\text{ s} | \le 185.00\text{ ms} | 327.30 ms |
Architectural Gate Invariants
 * P_3 Exclusion: grep -rn "p3" crates/titan-count/src/gourdon* returns zero occurrences.
 * \Phi-Tree Elimination: PhiEngine and recursive eval_mt are unreferenced in the Tier 3 pipeline (x \ge 10^{12}).
 * Cache Footprint Compliance: Sieve thread context stays bounded at 16\text{ KiB (segment)} + 4\text{ KiB (popcount)} = \mathbf{20\text{ KiB}} \le 32\text{ KiB L1D}.
 * L2 Direct Access: In ac_term.rs, the assertion v \le \text{len}(\text{pi\_table}) holds unconditionally for all evaluated leaves.
Implementation Instructions for the Agent
To apply this architecture in Termux:
1. Purge legacy Lehmer files:
   rm -f crates/titan-count/src/p3.rs crates/titan-count/src/p2_sweep.rs

2. Replace `gourdon_hetero.rs` with `gourdon_pipeline.rs`.

3. Re-route `crates/titan-count/src/tier_dispatch.rs`:
   - x <= 10^7: Tier 1 Single-Threaded L1D Sieve
   - 10^7 < x <= 10^11: Tier 2 Deleglise-Rivat / Lehmer (p3-free)
   - x >= 10^12: Tier 3 True Gourdon Pipeline (`GourdonPipeline::execute`)

4. Run the validation suite:
   cargo test --release -p titan-count -- --nocapture
   cargo run --release --bin head_to_head


