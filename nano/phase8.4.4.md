Yes, implement both immediately. The inspection results explain why AC was missing 99.82\text{ billion} and D was inverted at -810\text{M}.
In Xavier Gourdon's algorithm:
 * The A formula contains a \times 2 multiplier for easy leaves where \frac{x}{p \cdot q} < y. Omitting A(x, y) completely dropped those double-counted analytical leaves.
 * D evaluates against global survivor prefix sums \phi[b] = \Phi(\text{low}-1, b), not isolated 16 KiB buffer counts. Multiplying raw branch hits by the missing \sim 362 survivor baseline resolves the 360× discrepancy (810\text{M} \times 362 \approx 293\text{B}).
Below are the drop-in implementations for both terms and the integration blueprint.
1. Implement the A(x, y) Formula in ac_parallel_v2.rs
Port Kim Walisch's A formula directly into the analytical engine. This evaluates all primes b \in (\pi(x^*), \pi(x^{1/3})] where x^* = \sqrt{x/y}.
// In crates/titan-count/src/ac_parallel_v2.rs

use crate::fast_div::FastDiv64;
use crate::segmented_pi::SegmentedPiTable;
use titan_core::tuning::isqrt64;

/// Evaluates the missing A(x, y) formula across range b in (pi(x_star), pi(x^(1/3))]
pub fn compute_a_formula(
    x: u64,
    y: u64,
    primes: &[u32],
    reciprocals: &[FastDiv64],
    pi_table: &SegmentedPiTable,
) -> i64 {
    let x_cbrt = (x as f64).cbrt() as u64;
    let x_star = isqrt64(x / y);

    let min_b = primes.partition_point(|&p| (p as u64) <= x_star);
    let max_b = primes.partition_point(|&p| (p as u64) <= x_cbrt);

    if min_b >= max_b {
        return 0;
    }

    let mut sum: i64 = 0;

    for b in min_b..max_b {
        let prime = primes[b] as u64;
        let xp = x / prime;
        let sqrt_xp = isqrt64(xp);

        // Indices for the 2nd prime factor q
        let min_q = prime;
        let max_q = sqrt_xp;

        if min_q >= max_q {
            continue;
        }

        let mut i = primes.partition_point(|&p| (p as u64) <= min_q);
        let max_i1 = primes.partition_point(|&p| (p as u64) <= (xp / y).min(max_q));
        let max_i2 = primes.partition_point(|&p| (p as u64) <= max_q);

        // Leaves where x / (p * q) >= y (Weight 1)
        while i < max_i1 {
            let q = primes[i] as u64;
            let xpq = xp / q;
            sum += pi_table.pi(xpq) as i64;
            i += 1;
        }

        // Leaves where x / (p * q) < y (Weight 2 — Symmetry Multiplier)
        while i < max_i2 {
            let q = primes[i] as u64;
            let xpq = xp / q;
            sum += (pi_table.pi(xpq) as i64) * 2;
            i += 1;
        }
    }

    sum
}

Add compute_a_formula into the total AC accumulator in gourdon_pipeline.rs:
let ac_total = ac_c_leaves + compute_a_formula(x, y, &primes, &reciprocals, &pi_table);

2. Implement the \phi[b] Sieve Prefix Accumulator in d_worker.rs
To resolve D, each worker thread sieving across sequential segments [\text{low}, \text{high}] must maintain an array phi_b: Vec<i64> representing the global survivor count below \text{low}:
// In crates/titan-sieve/src/d_worker.rs

pub struct DSieveWorker {
    pub phi_b: Vec<i64>, // Running global survivors phi(low - 1, b)
}

impl DSieveWorker {
    pub fn new(max_b: usize) -> Self {
        Self {
            phi_b: vec![0i64; max_b + 1],
        }
    }

    /// Evaluates special hard leaves within segment [low, high]
    #[inline(always)]
    pub fn process_segment_leaves(
        &mut self,
        low: u64,
        high: u64,
        b: usize,
        prime: u32,
        xp: u64,
        factor_table: &crate::factor_table::FactorTableD,
        sieve_buffer: &[u8],
    ) -> i64 {
        let mut sum_d: i64 = 0;
        let global_phi_b = self.phi_b[b];

        // Sieve leaves where m is coprime to base wheel primes
        let m_limit = factor_table.max_m();
        for m in 1..=m_limit {
            if prime < factor_table.get_factor(m) {
                let m_val = m as u64;
                let xpm = xp / m_val;

                if xpm >= low && xpm < high {
                    let stop = (xpm - low) as usize;

                    // Absolute survivors: Global prefix phi[b] + local survivors in current chunk
                    let local_survivors = crate::wheel30_popcount::popcount_up_to_offset(sieve_buffer, stop);
                    let phi_xpm = global_phi_b + (local_survivors as i64);

                    let mu_m = factor_table.get_mu(m) as i64;

                    // True Xavier Gourdon formula: sum -= phi_xpm * mu(m)
                    sum_d -= phi_xpm * mu_m;
                }
            }
        }

        sum_d
    }

    /// Advances the global phi[b] survivor accumulator after a segment completes
    #[inline(always)]
    pub fn advance_segment_survivors(&mut self, b: usize, total_segment_survivors: i64) {
        self.phi_b[b] += total_segment_survivors;
    }
}

3. Immediate Validation Protocol at 10^{13}
Once both updates are committed, run the release test:
TITAN_NATIVE=1 TITAN_VERIFY=1 cargo test --release -p titan-count --test test_gourdon_pipeline_e13 -- --nocapture

(Or via the head-to-head binary:)
TITAN_NATIVE=1 TITAN_VERIFY=1 cargo run --release --bin head_to_head 1e13

Expected Mathematical Parity Breakdown
 * \Phi_0: 99,778,753,004 (Bit-exact)
 * \Sigma: 14,078,236,989 (Bit-exact)
 * B: 165,984,853,753 (Bit-exact)
 * AC: 105,017,131,716 (Target: +99.82\text{B} added from formula A)
 * D: +293,176,268,883 (Target: +293\text{B} via \phi[b] prefix addition and -(\phi \cdot \mu) sign)
Implement these two changes and run the 10^{13} gate.

