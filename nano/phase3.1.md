Phase 1: The Reciprocal Division Engine (umulh for AC(x, y, z))
In Xavier Gourdon's algorithm, the easy special leaves AC(x, y, z) require evaluating the quotient:
across hundreds of millions of (m, p) pairs. On the Cortex-A55 (6 cores of the SM4450), the hardware integer divider (udiv) takes 14 to 20 cycles and is non-pipelined—stalling the in-order execution pipeline. On the Cortex-A78 (2 cores), udiv takes 8 to 12 cycles with limited throughput.
Phase 1 replaces hardware division with 64-bit Granlund-Montgomery invariant reciprocal multiplication (umulh + lsr), executing in 2 cycles on ARM64 with full pipelining.
1. Mathematical Derivation & The N_{\max} < 2^{62} Bound
In the standard Granlund-Montgomery (1994) algorithm for arbitrary 64-bit unsigned division (n \in [0, 2^{64}-1]), approximately 30% of divisors require an add/sub compensation sequence because the ideal multiplier M \ge 2^{64}.
However, in Titan's prime counting engine, the dividend is:
For all benchmark scales up to x = 10^{18} < 2^{60}, n is strictly bounded by N_{\max} = 2^{62} \ll 2^{64}.
Theorem: Universal 2-Instruction Exact Reciprocal
For any prime divisor d \ge 2 and any dividend n < N_{\max} \le 2^{62}, there exists an integer shift s and a 64-bit multiplier M < 2^{64} such that:
Proof:
 * Selection of Shift s:
   Choose s such that 2^s \in [\frac{1}{2}d, d). Because every interval [\frac{1}{2}d, d) contains exactly one power of 2, set:
   
   
   Since 2^s < d, it follows that:
   
 * Multiplier Formulation:
   Define the ceiling multiplier:
   
   
   Let r = (d \cdot M - 2^{64+s}). By definition of the ceiling, 0 \le r < d, so:
   
   
   Because \frac{2^{64+s}}{d} < 2^{64} and d \ge 2, M \le 2^{64}-1. Thus, M fits in a standard unsigned 64-bit integer (u64).
 * Error Bound Verification:
   Let n = q \cdot d + k, where q = \lfloor n/d \rfloor and 0 \le k \le d - 1. Multiplying by M:
   
   
   For the floor \lfloor \frac{n \cdot M}{2^{64+s}} \rfloor to equal q, the fractional error must satisfy:
   
   
   Since k \le d - 1 and (d - 1)M = 2^{64+s} + r - M:
   
   
   Therefore, \delta < 1 if and only if M - (q + 1)r > 0.
   * Since 2^s \ge \frac{1}{2}d, M \ge \frac{2^{64} \cdot 2^s}{d} \ge 2^{63}.
   * Since n < N_{\max} \le 2^{62} and r < d:
     
   * Hence:
     
The fractional error is strictly bounded within [0, 1) for all n < N_{\max}, guaranteeing exact mathematical parity using only one multiplication and one shift.
2. Cache Footprint of Precomputed Invariant Primes
The divisor p in AC(x, y, z) is bounded by z:
| Scale (x) | Sieve Limit z | Number of Primes \pi(z) | Table Footprint (u64 + u8 packed into 16 B) | Cache Placement on SM4450 |
|---|---|---|---|---|
| 10^{12} | 24,820 | 2,730 | 43.68 KiB | Resident in Cortex-A78 L1D (64 KiB) |
| 10^{13} | 53,000 | 5,420 | 86.72 KiB | Fits in Cortex-A78 L2 (512 KiB) |
| 10^{14} | 113,200 | 10,723 | 171.56 KiB | Fits in Cortex-A78 L2 (512 KiB) |
| 10^{16} | 500,000 | 41,538 | 664.60 KiB | Resident in shared DynamIQ L3 cache |
| 10^{18} | 2,500,000 | 183,072 | 2.92 MiB | Streams sequentially through L3 cache |
3. Production Implementation: magic_reciprocal.rs
Create crates/titan-count/src/magic_reciprocal.rs:
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct FastDiv64 {
    pub mul: u64,
    pub shift: u8,
    pub is_direct: u8,
    pub _pad: [u8; 6],
}

impl FastDiv64 {
    /// Computes the exact multiplier and shift for prime divisor `d`.
    /// Guaranteed exact for any dividend n <= max_n.
    pub fn new(d: u64, max_n: u64) -> Self {
        assert!(d >= 2, "Divisor must be >= 2");

        // Power of 2 check
        if d.is_power_of_two() {
            return Self {
                mul: 0,
                shift: d.trailing_zeros() as u8,
                is_direct: 2,
                _pad: [0; 6],
            };
        }

        let l = 64 - (d - 1).leading_zeros(); // ceil(log2(d))
        let s = l - 1; // 2^s in [d/2, d)

        // Compute M = ceil(2^(64 + s) / d)
        // 2^(64 + s) = 2^s * 2^64
        let two_64_rem = ((1u128 << 64) % (d as u128)) as u64;
        let mut rem = (two_64_rem as u128) << s;
        let base_quot = ((1u128 << (64 + s)) / (d as u128)) as u64;
        let rem_d = (rem % (d as u128)) as u64;

        let (mul, is_direct) = if rem_d == 0 {
            (base_quot, 1)
        } else {
            let m = base_quot + 1;
            // Validate if direct umulh+lsr holds for max_n
            // Condition: M - (max_n / d + 1) * r > 0
            let r = d - rem_d;
            let q_max = max_n / d;
            let subtrahend = (q_max as u128 + 1) * (r as u128);
            if (m as u128) > subtrahend {
                (m, 1) // Pure 2-instruction sequence
            } else {
                // Fallback to standard Granlund-Montgomery for ultra-scales > 2^62
                let s_full = l;
                let m_full = (((1u128 << (64 + s_full)) + (d as u128 - 1)) / (d as u128)) as u64;
                (m_full, 0)
            }
        };

        Self {
            mul,
            shift: s as u8,
            is_direct,
            _pad: [0; 6],
        }
    }

    /// Evaluates floor(n / d) using ARM64 umulh
    #[inline(always)]
    pub fn div(&self, n: u64) -> u64 {
        if self.is_direct == 2 {
            return n >> self.shift;
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            if self.is_direct == 1 {
                let hi: u64;
                std::arch::asm!(
                    "umulh {hi}, {n}, {mul}",
                    "lsr {res}, {hi}, {shift}",
                    n = in(reg) n,
                    mul = in(reg) self.mul,
                    shift = in(reg) self.shift as u64,
                    hi = out(reg) hi,
                    res = lateout(reg) hi,
                    options(pure, nomem, nostack)
                );
                return hi;
            }
        }

        // Generic / Fallback path
        let hi = ((n as u128 * self.mul as u128) >> 64) as u64;
        if self.is_direct == 1 {
            hi >> self.shift
        } else {
            // Standard Granlund-Montgomery overflow compensation
            let t = ((n.wrapping_sub(hi)) >> 1).wrapping_add(hi);
            t >> (self.shift)
        }
    }
}

pub struct FastDivTable {
    table: Vec<FastDiv64>,
}

impl FastDivTable {
    pub fn build(primes: &[u32], max_n: u64) -> Self {
        let mut table = Vec::with_capacity(primes.len());
        for &p in primes {
            table.push(FastDiv64::new(p as u64, max_n));
        }
        Self { table }
    }

    #[inline(always)]
    pub fn get(&self, idx: usize) -> &FastDiv64 {
        unsafe { self.table.get_unchecked(idx) }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[FastDiv64] {
        &self.table
    }
}

4. 4-Way Pipelined Vector Unrolling in ac_term.rs
Integrating FastDiv64 with 4-way instruction-level loop unrolling enables dual-issue umulh execution on the Cortex-A78 ALU pipes while interleaving L1D/L2 PiTable lookups.
Update crates/titan-count/src/ac_term.rs:
use std::sync::atomic::{AtomicI64, Ordering};
use rayon::prelude::*;
use crate::magic_reciprocal::{FastDiv64, FastDivTable};

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

/// Evaluates Fused Leaves AC(x, y, z) using 64-bit Reciprocal Division (umulh)
pub fn compute_ac_fused(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u32],
    pi_table: &[u32],
    mu: &[i8],
    div_table: &FastDivTable,
) -> i64 {
    let ac_sum = AtomicI64::new(0);

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
        let mut i = p_start_idx;
        let div_slice = div_table.as_slice();

        // 4-Way Pipelined ILP Unrolling
        while i + 4 <= p_end_idx {
            let d0 = unsafe { div_slice.get_unchecked(i) };
            let d1 = unsafe { div_slice.get_unchecked(i + 1) };
            let d2 = unsafe { div_slice.get_unchecked(i + 2) };
            let d3 = unsafe { div_slice.get_unchecked(i + 3) };

            // Dual ALU-pipe execution: umulh instructions execute concurrently
            let v0 = d0.div(x_div_m);
            let v1 = d1.div(x_div_m);
            let v2 = d2.div(x_div_m);
            let v3 = d3.div(x_div_m);

            // Parallel L1/L2 Cache Lookups
            let pi0 = unsafe { *pi_table.get_unchecked(v0 as usize) as i64 };
            let pi1 = unsafe { *pi_table.get_unchecked(v1 as usize) as i64 };
            let pi2 = unsafe { *pi_table.get_unchecked(v2 as usize) as i64 };
            let pi3 = unsafe { *pi_table.get_unchecked(v3 as usize) as i64 };

            let pi_primes = ((i + 1) + (i + 2) + (i + 3) + (i + 4)) as i64;
            local_sum += (pi0 + pi1 + pi2 + pi3) - pi_primes + 4;

            i += 4;
        }

        // Tail cleanup
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

5. Verification Harness (tests/test_magic_reciprocal.rs)
To guarantee bit-exact correctness across edge cases, run this test suite in Termux:
#[cfg(test)]
mod tests {
    use crate::magic_reciprocal::FastDiv64;

    #[test]
    fn test_reciprocal_exhaustion_small_primes() {
        let test_primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 65537, 1000003];
        let max_n = 10_000_000_000_000_000u64; // 10^16

        for &p in &test_primes {
            let fast_div = FastDiv64::new(p, max_n);

            // Boundary values
            let test_values = [
                0, 1, p - 1, p, p + 1, 2 * p, 2 * p - 1,
                100, 1_000, 1_000_000, 1_000_000_000,
                max_n - 1, max_n
            ];

            for &n in &test_values {
                let expected = n / p;
                let actual = fast_div.div(n);
                assert_eq!(actual, expected, "Mismatch for n = {}, p = {}", n, p);
            }

            // Pseudo-random pseudo-sweep
            let mut state = 0x123456789ABCDEF0u64;
            for _ in 0..100_000 {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let n = state % max_n;
                assert_eq!(fast_div.div(n), n / p, "Random failure: n = {}, p = {}", n, p);
            }
        }
    }
}

Integration Steps
 * Place magic_reciprocal.rs under crates/titan-count/src/.
 * Register the module in crates/titan-count/src/lib.rs:
   pub mod magic_reciprocal;

 * Update GourdonPipeline::new in gourdon_pipeline.rs to build FastDivTable once during initialization and pass its reference to compute_ac_fused.
 * Run validation in Termux:
   cargo test --release -p titan-count -- test_reciprocal
cargo run --release --bin head_to_head


