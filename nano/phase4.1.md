Phase 4.1: Linear Factor Table for Greatest Prime Factor (\text{gpf})
In Xavier Gourdon's algorithm, the ordinary and easy special leaves AC(x, y, z) iterate over square-free integers m \le y. For each m, the inner prime loop must be strictly bounded from below by the greatest prime factor of m:
In the current codebase, gpf(m) evaluates this via trial division over primes up to \sqrt{m}.
1. Mathematical Derivation: Euler's Linear Sieve for \text{gpf}
At x = 10^{16}, y \approx 280,000. There are \approx 170,220 square-free integers m \le y.
 * With trial division, testing primes up to \sqrt{m} \le 529 requires up to 99 division steps (udiv) per integer.
 * On the Cortex-A55, each non-pipelined udiv consumes 14–20 cycles.
 * Across all m \le y, trial division burns \approx 255\times 10^6 CPU cycles (>120\text{ ms} of un-pipelined hardware stalls).
By replacing trial division with Euler's Linear Sieve, \text{gpf}(m) is precomputed for all m \le y in strictly O(y) operations with zero division instructions.
Euler's Sieve Transition:
n = p · i   where p = lpf(n) and p ≤ lpf(i)
┌─────────────────────────────────────────────────────────┐
│ Since p ≤ lpf(i) ≤ gpf(i), all prime factors of i are  │
│ greater than or equal to p.                             │
│ Therefore:                                              │
│               gpf(p · i) = gpf(i)                       │
│ Every composite integer is visited exactly ONCE.       │
└─────────────────────────────────────────────────────────┘

Theorem: Invariant \text{gpf} Propagation
For any composite integer n > 1, write n = p \cdot i where p = \text{lpf}(n). If p \le \text{lpf}(i), then:
Proof:
 * Since p = \text{lpf}(n), p is prime and divides n.
 * For all prime factors q \mid i, q \ge \text{lpf}(i) \ge p.
 * The set of prime factors of n = p \cdot i is \{p\} \cup \{q : q \mid i\}.
 * Because p \le q for all q \mid i:
   
This recurrence allows every composite n \le y to inherit its \text{gpf} in O(1) time during the sieve pass.
2. Cache Footprint and Memory Hierarchy
Because \text{gpf}(m) \le y, for all x \le 10^{20} (y \le 4.64\times 10^6 < 2^{32}), each entry fits into a 32-bit unsigned integer (u32).
| Scale (x) | Smooth Bound (y) | Array Length (y + 1) | Footprint (u32) | Placement on SM4450 |
|---|---|---|---|---|
| 10^{12} | 12,410 | 12,411 | 49.6 KiB | Cortex-A78 L1D (64 KiB) |
| 10^{13} | 53,000 | 53,001 | 212.0 KiB | Cortex-A78 L2 (512 KiB) |
| 10^{14} | 113,200 | 113,201 | 452.8 KiB | Cortex-A78 L2 (512 KiB) |
| 10^{16} | 280,000 | 280,001 | 1.12 MiB | Shared DynamIQ L3 (2 MiB) |
| 10^{18} | 1,250,000 | 1,250,001 | 5.00 MiB | Streams sequentially via L3 |
3. Complete Implementation: factor_table.rs
Create crates/titan-count/src/factor_table.rs:
// crates/titan-count/src/factor_table.rs

#[repr(C, align(64))]
pub struct FactorTable {
    gpf: Vec<u32>,
    max_y: usize,
}

impl FactorTable {
    /// Builds the greatest prime factor table for all m <= max_y in O(max_y) time
    /// using Euler's linear sieve. Zero hardware divisions.
    pub fn new(max_y: usize) -> Self {
        let mut gpf = vec![0u32; max_y + 1];
        let mut lpf = vec![0u32; max_y + 1];
        let mut primes = Vec::with_capacity(max_y / 8);

        for i in 2..=max_y {
            if lpf[i] == 0 {
                lpf[i] = i as u32;
                gpf[i] = i as u32;
                primes.push(i as u32);
            }

            let lpf_i = lpf[i];
            let gpf_i = gpf[i];

            for &p in &primes {
                if p > lpf_i {
                    break;
                }
                let next = i * (p as usize);
                if next > max_y {
                    break;
                }
                lpf[next] = p;
                // Invariant: p <= lpf[i] <= gpf[i], so largest factor is unchanged
                gpf[next] = gpf_i;
            }
        }

        // Drop lpf and primes immediately; only gpf is retained in memory
        Self { gpf, max_y }
    }

    /// O(1) lookup of greatest prime factor. Compiles to a single AArch64 LDR.
    #[inline(always)]
    pub fn gpf(&self, m: u64) -> u64 {
        if m <= 1 {
            return 0;
        }
        debug_assert!((m as usize) <= self.max_y, "m exceeds precomputed max_y");
        unsafe { *self.gpf.get_unchecked(m as usize) as u64 }
    }

    #[inline(always)]
    pub fn max_y(&self) -> usize {
        self.max_y
    }
}

4. Integration into ac_term.rs
Update crates/titan-count/src/ac_term.rs to delete the trial division function and read from FactorTable:
// crates/titan-count/src/ac_term.rs
use std::sync::atomic::{AtomicI64, Ordering};
use rayon::prelude::*;
use crate::magic_reciprocal::FastDivTable;
use crate::factor_table::FactorTable;

/// Evaluates Fused Leaves AC(x, y, z) using precomputed GPF lookups and umulh reciprocals
pub fn compute_ac_fused(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u32],
    pi_table: &[u32],
    mu: &[i8],
    div_table: &FastDivTable,
    factor_table: &FactorTable,
) -> i64 {
    let ac_sum = AtomicI64::new(0);

    (1..=y).into_par_iter().for_each(|m| {
        let mu_m = unsafe { *mu.get_unchecked(m as usize) };
        if mu_m == 0 { return; }

        // O(1) GPF resolution: replaces 14-20 cycle hardware udiv loop
        let gpf_m = factor_table.gpf(m);

        let x_div_m = x / m;
        let p_min_bound = (x_div_m / z).max(gpf_m);
        let p_max_bound = (x_div_m as f64).sqrt() as u64;

        if p_min_bound >= p_max_bound { return; }

        let p_start_idx = primes.partition_point(|&p| (p as u64) <= p_min_bound);
        let p_end_idx = primes.partition_point(|&p| (p as u64) <= p_max_bound);

        let mut local_sum: i64 = 0;
        let mut i = p_start_idx;
        let div_slice = div_table.as_slice();

        // 4-Way Pipelined ILP Unrolling using reciprocal division
        while i + 4 <= p_end_idx {
            let d0 = unsafe { div_slice.get_unchecked(i) };
            let d1 = unsafe { div_slice.get_unchecked(i + 1) };
            let d2 = unsafe { div_slice.get_unchecked(i + 2) };
            let d3 = unsafe { div_slice.get_unchecked(i + 3) };

            let v0 = d0.div(x_div_m);
            let v1 = d1.div(x_div_m);
            let v2 = d2.div(x_div_m);
            let v3 = d3.div(x_div_m);

            let pi0 = unsafe { *pi_table.get_unchecked(v0 as usize) as i64 };
            let pi1 = unsafe { *pi_table.get_unchecked(v1 as usize) as i64 };
            let pi2 = unsafe { *pi_table.get_unchecked(v2 as usize) as i64 };
            let pi3 = unsafe { *pi_table.get_unchecked(v3 as usize) as i64 };

            let pi_primes = ((i + 1) + (i + 2) + (i + 3) + (i + 4)) as i64;
            local_sum += (pi0 + pi1 + pi2 + pi3) - pi_primes + 4;

            i += 4;
        }

        while i < p_end_idx {
            let d = unsafe { div_slice.get_unchecked(i) };
            let v = d.div(x_div_m);
            let pi_v = unsafe { *pi_table.get_unchecked(v as usize) as i64 };
            let pi_p = (i + 1) as i64;
            local_sum += pi_v - pi_p + 1;
            i += 1;
        }

        if mu_m == 1 {
            ac_sum.fetch_add(local_sum, Ordering::Relaxed);
        } else {
            ac_sum.fetch_sub(local_sum, Ordering::Relaxed);
        }
    });

    ac_sum.load(Ordering::Relaxed)
}

5. Assembly Inspection: AArch64 Cycle Comparison
Legacy gpf(m) Trial Division Loop (Before)
.LBB0_1:
    ldr     x11, [x1, x10, lsl #3]   // Load prime p
    mul     x12, x11, x11            // p * p
    cmp     x12, x0                  // if p * p > n break
    b.hi    .LBB0_4
    udiv    x12, x0, x11             // Hardware 64-bit divide (14-20 cycles!)
    msub    x13, x12, x11, x0        // Remainder = n - (n/p)*p
    cbnz    x13, .LBB0_3             // Loop if not divisible
    // ... Inner while loop repeating udiv for prime powers ...

 * Cost: 15–30 cycles per prime checked \times up to 99 primes = up to 1,500 cycles per m.
FactorTable::gpf(m) (After)
    ldr     w0, [x1, x0, lsl #2]     // Load 32-bit GPF from base pointer x1

 * Cost: 1 instruction, 1 cycle (L1/L2 cache hit).
6. Verification Test Suite
Create crates/titan-count/tests/test_factor_table.rs:
// crates/titan-count/tests/test_factor_table.rs
use titan_count::factor_table::FactorTable;

fn naive_gpf(mut n: u64) -> u64 {
    if n <= 1 { return 0; }
    let mut max_p = 0;
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            max_p = max_p.max(d);
            while n % d == 0 { n /= d; }
        }
        d += 1;
    }
    if n > 1 { max_p = max_p.max(n); }
    max_p
}

#[test]
fn test_factor_table_exhaustive_parity() {
    let limit = 500_000;
    let table = FactorTable::new(limit);

    assert_eq!(table.gpf(0), 0);
    assert_eq!(table.gpf(1), 0);

    for m in 2..=limit as u64 {
        let expected = naive_gpf(m);
        let actual = table.gpf(m);
        assert_eq!(
            actual, expected,
            "GPF mismatch for m = {}: expected {}, got {}",
            m, expected, actual
        );
    }
}

Integration Steps
 * Register the new module in crates/titan-count/src/lib.rs:
   pub mod factor_table;

 * In gourdon_pipeline.rs:
   * Construct FactorTable::new(y as usize) once during pipeline initialization.
   * Pass &factor_table into compute_ac_fused.
 * Run verification in Termux:
   cargo test --release -p titan-count --test test_factor_table
cargo test --workspace --release
cargo run --release --bin head_to_head


