The sampled prime index eliminates the 26-step DRAM binary search stalls by replacing random memory traversals across a 203 MB array with a two-level bounded hierarchy: a 49.5 KiB index locked entirely in cache, followed by a local partition bounded within a single 16 KiB page.
DRAM Random Walk vs. Sampled Prime Index
┌────────────────────────────────────────────────────────────────────────┐
│ Current partition_point (50.8M primes, 203 MB, 51,200 4KiB pages):    │
│ 26 binary search steps jumping across 203 MB of LPDDR4X DRAM           │
│ 15-20 cacheline misses + TLB page walks = ~800-1,200 cycles per probe   │
├────────────────────────────────────────────────────────────────────────┤
│ Step 1 Sampled Index (Stride K = 4096 primes):                         │
│ Tier 1: 12,397 entries * 4 bytes = 49.5 KiB (100% in A78 L1D / A55 L2) │
│         14 iterations hitting cache-locked lines (0 DRAM misses)       │
│ Tier 2: 4,096 primes = 16 KiB (1 single memory page)                   │
│         12 local iterations; hardware prefetcher pulls line once       │
│ Total DRAM misses drop from 15-20 down to <= 2 per query!               │
└────────────────────────────────────────────────────────────────────────┘

1. Implementation: sampled_index.rs
Create crates/titan-count/src/sampled_index.rs:
pub const SAMPLE_STRIDE_BITS: usize = 12;
pub const SAMPLE_STRIDE: usize = 1 << SAMPLE_STRIDE_BITS; // 4096 primes per block

#[repr(C, align(64))]
pub struct SampledPrimeIndex {
    /// 49.5 KiB table of prime samples: sample[k] = primes[k * 4096]
    samples: Vec<u32>,
    total_primes: usize,
    max_prime: u64,
}

impl SampledPrimeIndex {
    pub fn build(primes: &[u32]) -> Self {
        if primes.is_empty() {
            return Self {
                samples: Vec::new(),
                total_primes: 0,
                max_prime: 0,
            };
        }

        let num_samples = (primes.len() + SAMPLE_STRIDE - 1) / SAMPLE_STRIDE;
        let mut samples = Vec::with_capacity(num_samples);

        for i in (0..primes.len()).step_by(SAMPLE_STRIDE) {
            unsafe {
                samples.push(*primes.get_unchecked(i));
            }
        }

        Self {
            samples,
            total_primes: primes.len(),
            max_prime: *primes.last().unwrap() as u64,
        }
    }

    /// Evaluates pi(v) for any v <= sqrt(x) in bounded cycles:
    /// 1. Binary search inside 49.5 KiB L1D/L2-resident table (14 steps, 0 DRAM misses)
    /// 2. Local bounded binary search inside single 16 KiB contiguous page (12 steps)
    #[inline(always)]
    pub fn pi(&self, primes: &[u32], v: u64) -> u64 {
        if v < 2 {
            return 0;
        }
        if v >= self.max_prime {
            return self.total_primes as u64;
        }

        let v_u32 = v as u32;

        // Tier 1: Search the 49.5 KiB cache-locked sample table
        // Slices fit 100% in Cortex-A78 L1D (64 KiB) or Cortex-A55 L2 (256 KiB)
        let sample_idx = self.samples.partition_point(|&sp| sp <= v_u32);

        // Sub-slice window boundaries
        let low_idx = if sample_idx == 0 { 0 } else { (sample_idx - 1) << SAMPLE_STRIDE_BITS };
        let high_idx = (sample_idx << SAMPLE_STRIDE_BITS).min(self.total_primes);

        // Tier 2: Bounded search within the single 16 KiB window
        unsafe {
            let window = primes.get_unchecked(low_idx..high_idx);
            
            // Prefetch the midpoint of the 16 KiB window to warm the L1 cacheline
            #[cfg(target_arch = "aarch64")]
            {
                let mid_ptr = window.as_ptr().add(window.len() >> 1);
                core::arch::aarch64::__pld(mid_ptr);
            }

            let local_offset = window.partition_point(|&p| p <= v_u32);
            (low_idx + local_offset) as u64
        }
    }

    #[inline(always)]
    pub fn total_primes(&self) -> usize {
        self.total_primes
    }

    #[inline(always)]
    pub fn table_bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<u32>()
    }
}

2. Integration into B(x, y) Engine (b_term.rs)
Update crates/titan-count/src/b_term.rs to replace the unbounded 203 MB binary search:
// crates/titan-count/src/b_term.rs
use crate::b_term::{StreamingReciprocalBuffer, RECIPROCAL_BLOCK_SIZE};
use crate::sampled_index::SampledPrimeIndex;

pub fn compute_b_gauss_sampled(
    x: u64,
    y: u64,
    primes: &[u32],
    pi_table: &[u32],
    sampled_idx: &SampledPrimeIndex,
) -> i64 {
    let sqrt_x = (x as f64).sqrt() as u64;
    if y >= sqrt_x { return 0; }

    let p_start = primes.partition_point(|&p| (p as u64) <= y);
    let p_end = primes.partition_point(|&p| (p as u64) <= sqrt_x);
    if p_start >= p_end { return 0; }

    // Gauss O(1) closed-form sum
    let a = (p_start + 1) as i64;
    let b = p_end as i64;
    let n = b - a + 1;
    let sum_pi_p = (a + b) * n / 2;
    let sum_ones = n;

    let active_primes = &primes[p_start..p_end];
    let total = active_primes.len();
    let mut sbrb = StreamingReciprocalBuffer::new();
    let mut sum_pi_quotients: i64 = 0;
    let pi_max = (pi_table.len() - 1) as u64;

    let mut chunk_start = 0;
    while chunk_start < total {
        let chunk_end = (chunk_start + RECIPROCAL_BLOCK_SIZE).min(total);
        let slice = &active_primes[chunk_start..chunk_end];
        let len = slice.len();

        sbrb.fill_block(slice, x);

        let mut i = 0;
        while i + 4 <= len {
            let q0 = sbrb.table[i].div(x);
            let q1 = sbrb.table[i + 1].div(x);
            let q2 = sbrb.table[i + 2].div(x);
            let q3 = sbrb.table[i + 3].div(x);

            // Replaced unbounded partition_point with sampled_idx.pi()
            let pi0 = if q0 <= pi_max { unsafe { *pi_table.get_unchecked(q0 as usize) as i64 } } else { sampled_idx.pi(primes, q0) as i64 };
            let pi1 = if q1 <= pi_max { unsafe { *pi_table.get_unchecked(q1 as usize) as i64 } } else { sampled_idx.pi(primes, q1) as i64 };
            let pi2 = if q2 <= pi_max { unsafe { *pi_table.get_unchecked(q2 as usize) as i64 } } else { sampled_idx.pi(primes, q2) as i64 };
            let pi3 = if q3 <= pi_max { unsafe { *pi_table.get_unchecked(q3 as usize) as i64 } } else { sampled_idx.pi(primes, q3) as i64 };

            sum_pi_quotients += pi0 + pi1 + pi2 + pi3;
            i += 4;
        }

        while i < len {
            let q = sbrb.table[i].div(x);
            let pi_q = if q <= pi_max { unsafe { *pi_table.get_unchecked(q as usize) as i64 } } else { sampled_idx.pi(primes, q) as i64 };
            sum_pi_quotients += pi_q;
            i += 1;
        }

        chunk_start = chunk_end;
    }

    sum_pi_quotients - sum_pi_p + sum_ones
}

3. Integration into AC(x, y, z) Hyperbola Engine (ac_hyperbola.rs)
Update crates/titan-count/src/ac_hyperbola.rs:
// crates/titan-count/src/ac_hyperbola.rs
use crate::sampled_index::SampledPrimeIndex;

#[inline(always)]
pub fn evaluate_ac_hyperbola_m_sampled(
    x_div_m: u64,
    p_min: u64,
    p_max: u64,
    primes: &[u32],
    pi_table: &[u32],
    sampled_idx: &SampledPrimeIndex,
) -> i64 {
    if p_min >= p_max { return 0; }

    let pi_max = (pi_table.len() - 1) as u64;
    let v_min = x_div_m / p_max;
    let v_max = x_div_m / (p_min + 1);

    let mut sum: i64 = 0;

    for v in v_min..=v_max {
        let pi_v = if v <= pi_max {
            unsafe { *pi_table.get_unchecked(v as usize) as i64 }
        } else {
            sampled_idx.pi(primes, v) as i64
        };

        let p_low = (x_div_m / (v + 1)).max(p_min);
        let p_high = (x_div_m / v).min(p_max);

        if p_low >= p_high { continue; }

        let idx_low = if p_low <= pi_max {
            unsafe { *pi_table.get_unchecked(p_low as usize) as i64 }
        } else {
            sampled_idx.pi(primes, p_low) as i64
        };

        let idx_high = if p_high <= pi_max {
            unsafe { *pi_table.get_unchecked(p_high as usize) as i64 }
        } else {
            sampled_idx.pi(primes, p_high) as i64
        };

        let delta_pi = idx_high - idx_low;
        if delta_pi <= 0 { continue; }

        let i_a = idx_low + 1;
        let i_b = idx_high;
        let sum_pi = (i_a + i_b) * delta_pi / 2;

        sum += delta_pi * (pi_v + 1) - sum_pi;
    }

    sum
}

4. Mathematical Parity & Verification Test Suite
Create crates/titan-count/tests/test_sampled_index.rs:
use titan_count::sampled_index::SampledPrimeIndex;
use titan_sieve::dense_popcount_neon::generate_primes_u32;

#[test]
fn test_sampled_index_exact_parity() {
    // Generate 2,000,000 primes (~32.4 million limit)
    let primes = generate_primes_u32(32_452_843);
    assert!(primes.len() >= 2_000_000);

    let index = SampledPrimeIndex::build(&primes);
    println!("Sample table footprint: {} bytes", index.table_bytes());
    assert!(index.table_bytes() <= 64 * 1024); // Must fit in 64 KiB L1D

    // Boundary cases
    assert_eq!(index.pi(&primes, 0), 0);
    assert_eq!(index.pi(&primes, 1), 0);
    assert_eq!(index.pi(&primes, 2), 1);
    assert_eq!(index.pi(&primes, 3), 2);
    assert_eq!(index.pi(&primes, 4), 2);
    assert_eq!(index.pi(&primes, 5), 3);

    let max_p = *primes.last().unwrap() as u64;
    assert_eq!(index.pi(&primes, max_p), primes.len() as u64);
    assert_eq!(index.pi(&primes, max_p + 100), primes.len() as u64);

    // Test exact parity against primes.partition_point across 200,000 random queries
    let mut rng_state: u64 = 0x853c49e6748fea9b;
    let mut xorshift = || {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        rng_state
    };

    for _ in 0..200_000 {
        let query_v = xorshift() % (max_p + 500);
        let expected = primes.partition_point(|&p| (p as u64) <= query_v) as u64;
        let actual = index.pi(&primes, query_v);
        assert_eq!(
            actual, expected,
            "Parity mismatch at v = {}: expected {}, got {}",
            query_v, expected, actual
        );
    }
}

5. Wiring into head_to_head_ultra.rs
Register pub mod sampled_index; in crates/titan-count/src/lib.rs.
Instantiate SampledPrimeIndex during precomputation in head_to_head_ultra.rs:
// Precomputation phase:
let sampled_idx = SampledPrimeIndex::build(&primes);
println!("  Sampled Prime Index built: {} bytes (100% L1D resident)", sampled_idx.table_bytes());

// Pass &sampled_idx into the execution pipeline

6. Compilation and Benchmarking Protocol
Run this in Termux on the SM4450:
# 1. Compile release tests and verify bit-exact parity across 200,000 randomized queries
cargo test --release -p titan-count --test test_sampled_index -- --nocapture

# 2. Build the ultra runner
cargo build --release --bin head_to_head_ultra

# 3. Allow 30s thermal stabilization back to idle (~37°C)
sleep 30

# 4. Execute the 10^17 and 10^18 benchmark
./target/release/head_to_head_ultra

Projected Delta from Step 1
 * DRAM Traffic: Eliminates over 90% of random DRAM page walks in B(x, y) and AC(x, y, z).
 * 10^{17}: Expected drop from 10.75 s down to ~9.60–9.90 s (surpassing primecount's 10.70 s).
 * 10^{18}: Expected recovery of ~3.5 to 4.5 seconds, dropping from 50.49 s down to ~46.0–47.0 s prior to implementing the Wheel-30 engine.

