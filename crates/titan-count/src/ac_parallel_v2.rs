//! Phase 8.3 & Phase 8.4.4: Dual-A78 Concurrent ILP-4 AC Engine + A(x, y) formula.
//!
//! Re-wires the lock-free AcWorkQueue so both Cortex-A78 cores run the 4-way ILP
//! unrolled reciprocal loop in parallel, plus evaluates the missing A(x, y) analytical leaves.

use std::sync::atomic::{AtomicU64, Ordering};
use crate::magic_reciprocal::{FastDiv64, FastDivTable};
use crate::pi_table::PiTable;
use crate::segmented_pi::SegmentedPiTable;
use titan_core::roots::{icbrt, isqrt};
use titan_core::tuning::{icbrt64, isqrt64};
use crate::sigma_l1::get_x_star_gourdon;

pub struct AcWorkQueue {
    current_m: AtomicU64,
    y: u64,
}

impl AcWorkQueue {
    pub fn new(y: u64) -> Self {
        Self {
            current_m: AtomicU64::new(1),
            y,
        }
    }

    #[inline(always)]
    pub fn claim_chunk(&self) -> Option<(u64, u64)> {
        let mut curr = self.current_m.load(Ordering::Relaxed);
        loop {
            if curr > self.y {
                return None;
            }
            let remaining = self.y - curr + 1;
            let step = (remaining / 64).clamp(16, 4096);
            let next = (curr + step).min(self.y + 1);

            match self.current_m.compare_exchange_weak(
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

pub fn compute_ac_range_ilp4(
    m_start: u64,
    m_end: u64,
    x: u64,
    z: u64,
    mu: &[i8],
    primes: &[u64],
    reciprocals: &[FastDiv64],
    pi_table: &SegmentedPiTable,
) -> i64 {
    let mut chunk_sum: i64 = 0;

    for m in m_start..m_end {
        let mu_m = if (m as usize) < mu.len() {
            unsafe { *mu.get_unchecked(m as usize) }
        } else {
            0
        };
        if mu_m == 0 {
            continue;
        }

        let x_div_m = x / m;
        let p_min_bound = x / (m * z);
        let p_max = isqrt(x_div_m);

        if p_min_bound >= p_max {
            continue;
        }

        let p_start_idx = primes.partition_point(|&p| p <= p_min_bound);
        let p_end_idx = primes.partition_point(|&p| p <= p_max);

        if p_start_idx >= p_end_idx {
            continue;
        }

        let mut sub_sum: i64 = 0;
        let mut idx = p_start_idx;

        while idx + 4 <= p_end_idx {
            let r0 = unsafe { reciprocals.get_unchecked(idx) };
            let r1 = unsafe { reciprocals.get_unchecked(idx + 1) };
            let r2 = unsafe { reciprocals.get_unchecked(idx + 2) };
            let r3 = unsafe { reciprocals.get_unchecked(idx + 3) };

            let v0 = r0.div(x_div_m);
            let v1 = r1.div(x_div_m);
            let v2 = r2.div(x_div_m);
            let v3 = r3.div(x_div_m);

            let pi_v0 = pi_table.pi(v0) as i64;
            let pi_v1 = pi_table.pi(v1) as i64;
            let pi_v2 = pi_table.pi(v2) as i64;
            let pi_v3 = pi_table.pi(v3) as i64;

            let p0 = (idx + 1) as i64;
            let p1 = (idx + 2) as i64;
            let p2 = (idx + 3) as i64;
            let p3 = (idx + 4) as i64;

            sub_sum += (pi_v0 - p0 + 1)
                     + (pi_v1 - p1 + 1)
                     + (pi_v2 - p2 + 1)
                     + (pi_v3 - p3 + 1);

            idx += 4;
        }

        while idx < p_end_idx {
            let v = unsafe { reciprocals.get_unchecked(idx).div(x_div_m) };
            let pi_v = pi_table.pi(v) as i64;
            let pi_p = (idx + 1) as i64;
            sub_sum += pi_v - pi_p + 1;
            idx += 1;
        }

        chunk_sum += (mu_m as i64) * sub_sum;
    }

    chunk_sum
}

/// Single-b kernel of [`compute_a_formula`]: A-leaves for one 0-based prime
/// index `b` into the sentinel-stripped slice `s`. Extracted verbatim so the
/// multi-threaded dispatcher shares the exact scalar code path.
///
/// The `rec` variant below replaces the single hardware division with an
/// exact reciprocal multiply; both must agree bit-for-bit (see parity tests).
#[inline]
pub fn compute_a_single_b(x: u64, y: u64, s: &[u64], b: usize, pi_table: &PiTable) -> i64 {
    let prime = s[b];
    if prime < 2 {
        return 0;
    }
    let xp = x / prime;
    let sqrt_xp = isqrt(xp);

    let min_q = prime;
    let max_q = sqrt_xp;

    if min_q >= max_q {
        return 0;
    }

    let mut sum: i64 = 0;
    let mut i = s.partition_point(|&p| p <= min_q);
    let max_i1 = s.partition_point(|&p| p <= (xp / y).min(max_q));
    let max_i2 = s.partition_point(|&p| p <= max_q);

    // Leaves where x / (p * q) >= y (Weight 1)
    while i < max_i1 {
        let q = s[i];
        let xpq = xp / q;
        sum += pi_table.pi(xpq) as i64;
        i += 1;
    }

    // Leaves where x / (p * q) < y (Weight 2 — Symmetry Multiplier)
    while i < max_i2 {
        let q = s[i];
        let xpq = xp / q;
        sum += (pi_table.pi(xpq) as i64) * 2;
        i += 1;
    }

    sum
}

/// Strike 4: reciprocal-division variant of [`compute_a_single_b`].
///
/// Bit-identical (FastDiv64 is exact for numerators `<= x <= 2^62`), but each
/// `xp / q` hardware division (12-20+ non-pipelined cycles on Cortex-A55)
/// becomes one `umulh` + `lsr` (~2 cycles, dual-issue).
/// `rec` must be a [`FastDivTable`] built over the SAME stripped slice `s`
/// with `max_n >= x`, so `rec[i]` is the reciprocal of `s[i]`.
#[inline]
pub fn compute_a_single_b_fast(
    x: u64,
    y: u64,
    s: &[u64],
    rec: &[FastDiv64],
    b: usize,
    pi_table: &PiTable,
) -> i64 {
    let prime = s[b];
    if prime < 2 {
        return 0;
    }
    let xp = x / prime;
    let sqrt_xp = isqrt(xp);

    let min_q = prime;
    let max_q = sqrt_xp;

    if min_q >= max_q {
        return 0;
    }

    let mut sum: i64 = 0;
    let mut i = s.partition_point(|&p| p <= min_q);
    let max_i1 = s.partition_point(|&p| p <= (xp / y).min(max_q));
    let max_i2 = s.partition_point(|&p| p <= max_q);

    // Leaves where x / (p * q) >= y (Weight 1)
    while i < max_i1 {
        let xpq = rec[i].div(xp);
        sum += pi_table.pi(xpq) as i64;
        i += 1;
    }

    // Leaves where x / (p * q) < y (Weight 2 — Symmetry Multiplier)
    while i < max_i2 {
        let xpq = rec[i].div(xp);
        sum += (pi_table.pi(xpq) as i64) * 2;
        i += 1;
    }

    sum
}

/// Evaluates the genuine Xavier Gourdon A(x, y) formula across range b in (pi(x_star), pi(x^(1/3))]
pub fn compute_a_formula(
    x: u64,
    y: u64,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    let x_cbrt = icbrt(x);
    let x_star = get_x_star_gourdon(x, y);
    let has_sentinel = primes.first() == Some(&0);
    let prime_slice = if has_sentinel { &primes[1..] } else { primes };

    let min_b = prime_slice.partition_point(|&p| p <= x_star);
    let max_b = prime_slice.partition_point(|&p| p <= x_cbrt);

    if min_b >= max_b {
        return 0;
    }

    let mut sum: i64 = 0;

    for b in min_b..max_b {
        sum += compute_a_single_b(x, y, prime_slice, b, pi_table);
    }

    sum
}

/// Evaluates C2 leaves for b in (pi(sqrt(z)), pi(x_star)].
///
/// Scalar reference: one division + one `pi()` per prime q.
/// Kept as the bit-exact baseline for the clustered port below.
///
/// Bounds mirror Walisch `C2` (primecount-ref/src/gourdon/AC.cpp:145-198)
/// collapsed over segments to full range:
/// * b-partition at Walisch `x_star` ([`get_x_star_gourdon`]) — using
///   `isqrt(x/y)` here would overlap A-leaves on `(x_star, sqrt(x/y)]` and
///   double-count the band;
/// * `min_q = max(p, xp/p^2, sqrt(x)/p)`: the `xp/p^2` term keeps the
///   `pi(xpq) - b + 2` shortcut exact (`xpq < p^2` required); `sqrt(x)/p`
///   is the segment-window union bound. A `z/p` floor does NOT belong here
///   (the reference has none in C2).
pub fn compute_c2_formula(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    let x_star = get_x_star_gourdon(x, y);
    let sqrt_z = isqrt(z);
    let sqrt_x = isqrt(x);
    let has_sentinel = primes.first() == Some(&0);
    let prime_slice = if has_sentinel { &primes[1..] } else { primes };

    let min_b = prime_slice.partition_point(|&p| p <= sqrt_z);
    let max_b = prime_slice.partition_point(|&p| p <= x_star);

    if min_b >= max_b {
        return 0;
    }

    let mut sum: i64 = 0;

    for b in min_b..max_b {
        let prime = prime_slice[b];
        if prime < 2 {
            continue;
        }
        let xp = x / prime;
        let xp_div_p = xp / prime;

        // Full-range C2 window: q in (min_q, max_q]. Overflow-free:
        // xp/p^2 == (xp/p)/p for integers; prime >= 2 so no div-by-zero.
        let min_q = prime.max(xp_div_p / prime).max(sqrt_x / prime);
        let max_q = xp_div_p.min(y);

        if min_q >= max_q {
            continue;
        }

        let mut i = prime_slice.partition_point(|&p| p <= min_q);
        let max_i = prime_slice.partition_point(|&p| p <= max_q);

        let b_1based = (b + 1) as i64;
        while i < max_i {
            let q = prime_slice[i];
            let xpq = xp / q;
            let phi_xpq = (pi_table.pi(xpq) as i64) - b_1based + 2;
            sum += phi_xpq;
            i += 1;
        }
    }

    sum
}

/// Phase 9.2.2 (Strike 2): run-length clustered C2 leaves.
///
/// Faithful full-range port of Walisch `C2`
/// (primecount-ref/src/gourdon/AC.cpp:145-198):
/// per b, the q-window splits at `min_clustered = clamp(isqrt(xp), min_q, max_q)`.
/// Sparse q (`<= min_clustered`) are evaluated one-by-one; clustered q use the
/// run-length accumulation `phi * (i - imin)` with zero primes-array access in
/// the descent except resolving the next span boundary.
///
/// The iteration space (b-range, `min_q`/`max_q`) is IDENTICAL to
/// [`compute_c2_formula`]; only the inner evaluation is clustered.
/// `&[u64]` + exact [`isqrt64`] throughout. Requires:
/// * `pi_table` spanning at least `x / z` (C2 queries satisfy
///   `xpq < x_star^2 <= x / z` on Tier-3 scales),
/// * `primes` reaching at least `y` (C2 spans `q <= y`).
pub fn compute_c2_clustered(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    if y < 2 || z < 2 {
        return 0;
    }
    let x_star = get_x_star_gourdon(x, y);
    let sqrt_z = isqrt64(z);
    let sqrt_x = isqrt64(x);
    let has_sentinel = primes.first() == Some(&0);
    let s: &[u64] = if has_sentinel { &primes[1..] } else { primes };
    if s.is_empty() {
        return 0;
    }

    let min_b = s.partition_point(|&p| p <= sqrt_z);
    let max_b = s.partition_point(|&p| p <= x_star);
    if min_b >= max_b {
        return 0;
    }

    // pi-count of v = number of primes <= v = 1-based index of largest prime <= v.
    #[inline(always)]
    fn picount(pi_table: &PiTable, v: u64) -> usize {
        pi_table.pi(v) as usize
    }

    let mut total: i64 = 0;

    for b in min_b..max_b {
        total += compute_c2_single_b(x, y, s, b, pi_table, sqrt_x);
    }

    total
}

/// Single-b kernel of [`compute_c2_clustered`]: clustered C2 leaves for one
/// 0-based prime index `b` into the sentinel-stripped slice `s`.
/// `sqrt_x = isqrt(x)` is hoisted by the caller. Extracted verbatim so the
/// multi-threaded dispatcher shares the exact scalar code path.
#[inline]
pub fn compute_c2_single_b(
    x: u64,
    y: u64,
    s: &[u64],
    b: usize,
    pi_table: &PiTable,
    sqrt_x: u64,
) -> i64 {
    #[inline(always)]
    fn picount(pi_table: &PiTable, v: u64) -> usize {
        pi_table.pi(v) as usize
    }

    let prime = s[b];
    if prime < 2 {
        return 0;
    }
    let xp = x / prime;
    let xp_div_p = xp / prime;

    // Full-range C2 window (identical to scalar): q in (min_q, max_q].
    let min_q = prime.max(xp_div_p / prime).max(sqrt_x / prime);
    let max_q = xp_div_p.min(y);
    if min_q >= max_q {
        return 0;
    }

    // 0-based index window: j in [j_lo, j_hi).
    let j_lo = picount(pi_table, min_q);
    let mut j_hi = picount(pi_table, max_q);
    if j_hi > s.len() {
        j_hi = s.len();
    }
    if j_lo >= j_hi {
        return 0;
    }

    // Cluster threshold: q above isqrt(xp) share quotients.
    let clustered_q = isqrt64(xp).clamp(min_q, max_q);
    let j_split = picount(pi_table, clustered_q).clamp(j_lo, j_hi);

    let b1 = (b + 1) as i64;
    let mut b_sum: i64 = 0;

    // 1. Sparse region q in (min_q, clustered_q]: distinct quotients expected.
    for j in j_lo..j_split {
        let xpq = xp / s[j];
        b_sum += pi_table.pi(xpq) as i64 - b1 + 2;
    }

    // 2. Clustered descent (Walisch): counts only, hang-proof guards.
    let mut i_count = j_hi; // exclusive prime-count bound
    while i_count > j_split {
        let q = s[i_count - 1];
        // q >= 2 guaranteed: q > min_q >= 2.
        let xpq = xp / q.max(1);
        let pi_xpq = picount(pi_table, xpq);
        let phi = pi_xpq as i64 - b1 + 2;
        if pi_xpq >= s.len() {
            // Prime slice too short to resolve the span: exact scalar step.
            b_sum += phi;
            i_count -= 1;
            continue;
        }
        // First prime strictly above xpq; all q' yielding the same xpq sit
        // at counts (imin, i_count].
        let xpq2 = xp / s[pi_xpq].max(1);
        let imin = picount(pi_table, xpq2.max(clustered_q)).min(i_count - 1);
        b_sum += phi * (i_count - imin) as i64;
        i_count = imin;
    }

    b_sum
}

/// Strike 4: reciprocal-division variant of [`compute_c2_single_b`].
///
/// Bit-identical (`rec[i]` reciprocal of `s[i]`, `max_n >= x`); all three
/// hardware divisions per descent step become `umulh`+`lsr`.
#[inline]
pub fn compute_c2_single_b_fast(
    x: u64,
    y: u64,
    s: &[u64],
    rec: &[FastDiv64],
    b: usize,
    pi_table: &PiTable,
    sqrt_x: u64,
) -> i64 {
    #[inline(always)]
    fn picount(pi_table: &PiTable, v: u64) -> usize {
        pi_table.pi(v) as usize
    }

    let prime = s[b];
    if prime < 2 {
        return 0;
    }
    let xp = x / prime;
    let xp_div_p = xp / prime;

    // Full-range C2 window (identical to scalar): q in (min_q, max_q].
    let min_q = prime.max(xp_div_p / prime).max(sqrt_x / prime);
    let max_q = xp_div_p.min(y);
    if min_q >= max_q {
        return 0;
    }

    // 0-based index window: j in [j_lo, j_hi).
    let j_lo = picount(pi_table, min_q);
    let mut j_hi = picount(pi_table, max_q);
    if j_hi > s.len() {
        j_hi = s.len();
    }
    if j_lo >= j_hi {
        return 0;
    }

    // Cluster threshold: q above isqrt(xp) share quotients.
    let clustered_q = isqrt64(xp).clamp(min_q, max_q);
    let j_split = picount(pi_table, clustered_q).clamp(j_lo, j_hi);

    let b1 = (b + 1) as i64;
    let mut b_sum: i64 = 0;

    // 1. Sparse region q in (min_q, clustered_q]: distinct quotients expected.
    for j in j_lo..j_split {
        let xpq = rec[j].div(xp);
        b_sum += pi_table.pi(xpq) as i64 - b1 + 2;
    }

    // 2. Clustered descent (Walisch): counts only, hang-proof guards.
    let mut i_count = j_hi; // exclusive prime-count bound
    while i_count > j_split {
        // rec indexes s identically: rec[k] is the reciprocal of s[k].
        let xpq = rec[i_count - 1].div(xp);
        let pi_xpq = picount(pi_table, xpq);
        let phi = pi_xpq as i64 - b1 + 2;
        if pi_xpq >= s.len() {
            // Prime slice too short to resolve the span: exact scalar step.
            b_sum += phi;
            i_count -= 1;
            continue;
        }
        // First prime strictly above xpq; all q' yielding the same xpq sit
        // at counts (imin, i_count].
        let xpq2 = rec[pi_xpq].div(xp);
        let imin = picount(pi_table, xpq2.max(clustered_q)).min(i_count - 1);
        b_sum += phi * (i_count - imin) as i64;
        i_count = imin;
    }

    b_sum
}

/// Recursive C1 kernel: squarefree-m enumeration with alternating Mobius sign.
///
/// Faithful port of Walisch `C1<-MU>` (primecount-ref/src/gourdon/AC.cpp:106-140).
/// `i` is a 1-based prime index into `s` (0-based slice, sentinel stripped);
/// `mu_sign` flips every depth level. Overflow-safe via `checked_mul`.
fn c1_rec(
    xp: u64,
    b1: i64,
    i: usize,
    pi_y: usize,
    m: u64,
    min_m: u64,
    max_m: u64,
    s: &[u64],
    pi_table: &PiTable,
    mu_sign: i64,
) -> i64 {
    let mut sum = 0i64;
    let mut i = i;
    while i <= pi_y {
        let p = match s.get(i - 1) {
            Some(&p) => p,
            None => return sum,
        };
        let m_next = match m.checked_mul(p) {
            Some(v) => v,
            None => return sum,
        };
        if m_next > max_m {
            return sum;
        }
        if m_next > min_m {
            // m_next >= 2 (m >= 1, p >= 2): division is safe.
            let xpm = xp / m_next.max(1);
            sum += mu_sign * (pi_table.pi(xpm) as i64 - b1 + 2);
        }
        sum += c1_rec(xp, b1, i + 1, pi_y, m_next, min_m, max_m, s, pi_table, -mu_sign);
        i += 1;
    }
    sum
}

/// Phase 9.2.2 (Strike 2): C1 leaves for b in (max(k, pi((x/z)^(1/3))), pi(sqrt(z))].
///
/// Faithful port of the C1 section of `AC_OpenMP`
/// (primecount-ref/src/gourdon/AC.cpp:240-259). This b-range is NOT covered by
/// [`compute_a_formula`] (b > pi(x_star)) or [`compute_c2_formula`]
/// (b > pi(sqrt(z))); without C1 no native AC sum can match FFI `AC = A - C1 + C2`.
/// `k` mirrors the FFI call (pipeline passes 8). Requires `pi_table` spanning
/// at least `z` and `primes` reaching at least `y`.
pub fn compute_c1_native(
    x: u64,
    y: u64,
    z: u64,
    k: usize,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    if y < 2 || z < 2 {
        return 0;
    }
    let has_sentinel = primes.first() == Some(&0);
    let s: &[u64] = if has_sentinel { &primes[1..] } else { primes };
    if s.is_empty() {
        return 0;
    }

    // 1-based b window [b_start, b_end]: b_start = max(k, pi((x/z)^(1/3))) + 1.
    let cbrt_x_div_z = icbrt64(x / z.max(1));
    let sqrt_z = isqrt64(z);
    let pi_cbrt = pi_table.pi(cbrt_x_div_z) as usize;
    let b_start = k.max(pi_cbrt) + 1;
    let b_end = pi_table.pi(sqrt_z) as usize; // inclusive 1-based
    if b_start > b_end {
        return 0;
    }

    let pi_y = pi_table.pi(y) as usize;
    let mut total: i64 = 0;

    // 0-based j = b - 1.
    for j in (b_start - 1)..b_end.min(s.len()) {
        let prime = s[j];
        if prime < 2 {
            continue;
        }
        let xp = x / prime;
        // Overflow-free forms: xp/(p*p) == (xp/p)/p for integers.
        let xp_div_p = xp / prime;
        let max_m = xp_div_p.min(z);
        let min_m128 = (xp_div_p / prime).max(z / prime);
        let min_m = min_m128.min(max_m);
        let b1 = (j + 1) as i64;
        // Walisch: sum -= C1<-1>(xp, b, b, pi_y, 1, min_m, max_m); i starts at b + 1.
        total += c1_rec(xp, b1, j + 2, pi_y, 1, min_m, max_m, s, pi_table, -1);
    }

    total
}

/// Phase 9.2.2 (Strike 2): full native AC assembler.
///
/// `AC(x, y, z) = A(x, y) - C1(x, y, z) + C2(x, y, z)`, mirroring the signs in
/// `AC_OpenMP` (`sum -= C1`, `sum += C2`, `sum += A`).
/// A-leaves use the scalar [`compute_a_formula`], which is already the exact
/// full-range equivalent of Walisch `A` (AC.cpp:53-90): its weight-1 loop is
/// provably empty when `p > sqrt(x/y)`, and its bounds match the reference
/// term-for-term — so clustering A would add risk for zero structural gain.
pub fn compute_ac_native(
    x: u64,
    y: u64,
    z: u64,
    k: usize,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    let a = compute_a_formula(x, y, primes, pi_table);
    let c1 = compute_c1_native(x, y, z, k, primes, pi_table);
    let c2 = compute_c2_clustered(x, y, z, primes, pi_table);
    a - c1 + c2
}

/// Phase 9.2.3 (Strike 3): dynamic chunk size for the DynamIQ dispatcher.
///
/// Small enough that the heaviest small-b region splits across workers
/// (chunks of consecutive b's: the first chunks hold a wildly disproportionate
/// share — 32-wide bundling strands 7 cores behind one straggler).
/// Coarse enough to keep atomic traffic (a few thousand claims) off the bus.
pub const AC_CHUNK_SIZE: usize = 8;

/// Phase 9.2.3 (Strike 3): A-leaves across DynamIQ cores.
///
/// Dynamic guided dispatch over the b-range: workers claim 32-b chunks via an
/// `AtomicUsize` cursor, so big cores naturally drain 3-4x more chunks than
/// little cores. Thread-local `i64` sums reduce once at scope end
/// (no hot-path atomics on the sum). Bit-identical to [`compute_a_formula`].
pub fn compute_a_parallel(
    x: u64,
    y: u64,
    primes: &[u64],
    pi_table: &PiTable,
    num_threads: usize,
) -> i64 {
    use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

    let x_cbrt = icbrt(x);
    let x_star = get_x_star_gourdon(x, y);
    let has_sentinel = primes.first() == Some(&0);
    let s: &[u64] = if has_sentinel { &primes[1..] } else { primes };

    let min_b = s.partition_point(|&p| p <= x_star);
    let max_b = s.partition_point(|&p| p <= x_cbrt);
    if min_b >= max_b {
        return 0;
    }

    let threads = num_threads.clamp(1, 32);
    let cursor = AtomicUsize::new(min_b);
    // Borrowck-safe accumulation: one atomic slot per worker (disjoint indices).
    let slots: Vec<AtomicI64> = (0..threads).map(|_| AtomicI64::new(0)).collect();
    // Shared borrows for `move` closures (cursor/slots stay owned here).
    let cursor_ref = &cursor;
    let slots_ref = &slots;

    std::thread::scope(|scope| {
        for tid in 0..threads {
            scope.spawn(move || {
                titan_pool::worker::bind_worker_affinity(tid);
                let mut local: i64 = 0;
                loop {
                    let start = cursor_ref.fetch_add(AC_CHUNK_SIZE, Ordering::Relaxed);
                    if start >= max_b {
                        break;
                    }
                    let end = (start + AC_CHUNK_SIZE).min(max_b);
                    for b in start..end {
                        local += compute_a_single_b(x, y, s, b, pi_table);
                    }
                }
                slots_ref[tid].store(local, Ordering::Relaxed);
            });
        }
    });

    slots.iter().map(|a| a.load(Ordering::Relaxed)).sum()
}

/// Phase 9.2.3 (Strike 3): clustered C2-leaves across DynamIQ cores.
///
/// Same dynamic dispatch as [`compute_a_parallel`] over the C2 b-range.
/// Bit-identical to [`compute_c2_clustered`].
pub fn compute_c2_parallel(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    pi_table: &PiTable,
    num_threads: usize,
) -> i64 {
    use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

    if y < 2 || z < 2 {
        return 0;
    }
    let x_star = get_x_star_gourdon(x, y);
    let sqrt_z = isqrt64(z);
    let sqrt_x = isqrt64(x);
    let has_sentinel = primes.first() == Some(&0);
    let s: &[u64] = if has_sentinel { &primes[1..] } else { primes };
    if s.is_empty() {
        return 0;
    }

    let min_b = s.partition_point(|&p| p <= sqrt_z);
    let max_b = s.partition_point(|&p| p <= x_star);
    if min_b >= max_b {
        return 0;
    }

    let threads = num_threads.clamp(1, 32);
    let cursor = AtomicUsize::new(min_b);
    let slots: Vec<AtomicI64> = (0..threads).map(|_| AtomicI64::new(0)).collect();
    // Shared borrows for `move` closures (cursor/slots stay owned here).
    let cursor_ref = &cursor;
    let slots_ref = &slots;

    std::thread::scope(|scope| {
        for tid in 0..threads {
            scope.spawn(move || {
                titan_pool::worker::bind_worker_affinity(tid);
                let mut local: i64 = 0;
                loop {
                    let start = cursor_ref.fetch_add(AC_CHUNK_SIZE, Ordering::Relaxed);
                    if start >= max_b {
                        break;
                    }
                    let end = (start + AC_CHUNK_SIZE).min(max_b);
                    for b in start..end {
                        local += compute_c2_single_b(x, y, s, b, pi_table, sqrt_x);
                    }
                }
                slots_ref[tid].store(local, Ordering::Relaxed);
            });
        }
    });

    slots.iter().map(|a| a.load(Ordering::Relaxed)).sum()
}

/// Phase 9.2.3 (Strike 3): full native AC, multi-threaded.
///
/// `AC = A_parallel - C1 + C2_parallel`. Phases run sequentially with the full
/// worker count each (no oversubscription, clean core affinity). C1 stays
/// single-threaded: its b-range is tiny (~76 iterations at 1e16) with shallow
/// recursion — dispatch overhead would exceed its total cost.
/// Bit-identical to [`compute_ac_native`].
pub fn compute_ac_native_mt(
    x: u64,
    y: u64,
    z: u64,
    k: usize,
    primes: &[u64],
    pi_table: &PiTable,
    num_threads: usize,
) -> i64 {
    let t_a = std::time::Instant::now();
    let a = compute_a_parallel(x, y, primes, pi_table, num_threads);
    println!("[TITAN-PERF] native A_parallel latency: {:?}", t_a.elapsed());
    let t_c2 = std::time::Instant::now();
    let c2 = compute_c2_parallel(x, y, z, primes, pi_table, num_threads);
    println!("[TITAN-PERF] native C2_parallel latency: {:?}", t_c2.elapsed());
    let t_c1 = std::time::Instant::now();
    let c1 = compute_c1_native(x, y, z, k, primes, pi_table);
    println!("[TITAN-PERF] native C1 latency: {:?}", t_c1.elapsed());
    a - c1 + c2
}

/// Strike 4: reciprocal-division A-leaves across DynamIQ cores.
///
/// Identical dispatch to [`compute_a_parallel`], but every `xp / q` hardware
/// division becomes one exact `umulh`+`lsr`. `rec` must align with the
/// sentinel-stripped prime slice. Bit-identical to [`compute_a_parallel`].
pub fn compute_a_parallel_fast(
    x: u64,
    y: u64,
    primes: &[u64],
    rec: &[FastDiv64],
    pi_table: &PiTable,
    num_threads: usize,
) -> i64 {
    use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

    let x_cbrt = icbrt(x);
    let x_star = get_x_star_gourdon(x, y);
    let has_sentinel = primes.first() == Some(&0);
    let s: &[u64] = if has_sentinel { &primes[1..] } else { primes };
    // Contract: rec aligns 1:1 with s (built over the same stripped slice
    // with max_n >= x). Loud panic on violation — never silent corruption.
    debug_assert!(rec.len() >= s.len(), "reciprocal table shorter than prime slice");
    let r: &[FastDiv64] = rec;

    let min_b = s.partition_point(|&p| p <= x_star);
    let max_b = s.partition_point(|&p| p <= x_cbrt);
    if min_b >= max_b {
        return 0;
    }

    let threads = num_threads.clamp(1, 32);
    let cursor = AtomicUsize::new(min_b);
    let slots: Vec<AtomicI64> = (0..threads).map(|_| AtomicI64::new(0)).collect();
    let cursor_ref = &cursor;
    let slots_ref = &slots;

    std::thread::scope(|scope| {
        for tid in 0..threads {
            scope.spawn(move || {
                titan_pool::worker::bind_worker_affinity(tid);
                let mut local: i64 = 0;
                loop {
                    let start = cursor_ref.fetch_add(AC_CHUNK_SIZE, Ordering::Relaxed);
                    if start >= max_b {
                        break;
                    }
                    let end = (start + AC_CHUNK_SIZE).min(max_b);
                    for b in start..end {
                        local += compute_a_single_b_fast(x, y, s, r, b, pi_table);
                    }
                }
                slots_ref[tid].store(local, Ordering::Relaxed);
            });
        }
    });

    slots.iter().map(|a| a.load(Ordering::Relaxed)).sum()
}

/// Strike 4: reciprocal-division clustered C2-leaves across DynamIQ cores.
///
/// Bit-identical to [`compute_c2_parallel`].
pub fn compute_c2_parallel_fast(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    rec: &[FastDiv64],
    pi_table: &PiTable,
    num_threads: usize,
) -> i64 {
    use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

    if y < 2 || z < 2 {
        return 0;
    }
    let x_star = get_x_star_gourdon(x, y);
    let sqrt_z = isqrt64(z);
    let sqrt_x = isqrt64(x);
    let has_sentinel = primes.first() == Some(&0);
    let s: &[u64] = if has_sentinel { &primes[1..] } else { primes };
    if s.is_empty() {
        return 0;
    }
    // Contract: rec aligns 1:1 with s (built over the same stripped slice
    // with max_n >= x). Loud panic on violation — never silent corruption.
    debug_assert!(rec.len() >= s.len(), "reciprocal table shorter than prime slice");
    let r: &[FastDiv64] = rec;

    let min_b = s.partition_point(|&p| p <= sqrt_z);
    let max_b = s.partition_point(|&p| p <= x_star);
    if min_b >= max_b {
        return 0;
    }

    let threads = num_threads.clamp(1, 32);
    let cursor = AtomicUsize::new(min_b);
    let slots: Vec<AtomicI64> = (0..threads).map(|_| AtomicI64::new(0)).collect();
    let cursor_ref = &cursor;
    let slots_ref = &slots;

    std::thread::scope(|scope| {
        for tid in 0..threads {
            scope.spawn(move || {
                titan_pool::worker::bind_worker_affinity(tid);
                let mut local: i64 = 0;
                loop {
                    let start = cursor_ref.fetch_add(AC_CHUNK_SIZE, Ordering::Relaxed);
                    if start >= max_b {
                        break;
                    }
                    let end = (start + AC_CHUNK_SIZE).min(max_b);
                    for b in start..end {
                        local += compute_c2_single_b_fast(x, y, s, r, b, pi_table, sqrt_x);
                    }
                }
                slots_ref[tid].store(local, Ordering::Relaxed);
            });
        }
    });

    slots.iter().map(|a| a.load(Ordering::Relaxed)).sum()
}

/// Strike 4: full native AC with reciprocal division, multi-threaded.
///
/// Builds ONE [`FastDivTable`] over the sentinel-stripped primes with
/// `max_n = x` (exact for every AC numerator) and shares it across all
/// workers. Bit-identical to [`compute_ac_native_mt`].
pub fn compute_ac_native_mt_fast(
    x: u64,
    y: u64,
    z: u64,
    k: usize,
    primes: &[u64],
    pi_table: &PiTable,
    num_threads: usize,
) -> i64 {
    let t_build = std::time::Instant::now();
    let stripped: &[u64] = if primes.first() == Some(&0) { &primes[1..] } else { primes };
    let div_table = FastDivTable::build(stripped, x);
    let rec = div_table.as_slice();
    println!("[TITAN-PERF] FastDivTable build latency: {:?}", t_build.elapsed());
    let t_a = std::time::Instant::now();
    let a = compute_a_parallel_fast(x, y, primes, rec, pi_table, num_threads);
    println!("[TITAN-PERF] native A_fast latency: {:?}", t_a.elapsed());
    let t_c2 = std::time::Instant::now();
    let c2 = compute_c2_parallel_fast(x, y, z, primes, rec, pi_table, num_threads);
    println!("[TITAN-PERF] native C2_fast latency: {:?}", t_c2.elapsed());
    let t_c1 = std::time::Instant::now();
    let c1 = compute_c1_native(x, y, z, k, primes, pi_table);
    println!("[TITAN-PERF] native C1 latency: {:?}", t_c1.elapsed());
    a - c1 + c2
}

/// Strike 4: full native AC with L1D-windowed A-leaves.
///
/// `AC = A_windowed - C1 + C2_fast`. `count_table` is the small exact table
/// (span `x_star^2`, for bounds + window base counts); `rec` the shared
/// reciprocal table. Bit-identical to [`compute_ac_native_mt_fast`].
pub fn compute_ac_native_mt_windowed(
    x: u64,
    y: u64,
    z: u64,
    k: usize,
    primes: &[u64],
    rec: &[FastDiv64],
    count_table: &PiTable,
    num_threads: usize,
) -> i64 {
    let t_a = std::time::Instant::now();
    let a = compute_a_windowed_mt(x, y, primes, rec, count_table, num_threads);
    println!("[TITAN-PERF] native A_windowed latency: {:?}", t_a.elapsed());
    let t_c2 = std::time::Instant::now();
    let c2 = compute_c2_parallel_fast(x, y, z, primes, rec, count_table, num_threads);
    println!("[TITAN-PERF] native C2_fast latency: {:?}", t_c2.elapsed());
    let t_c1 = std::time::Instant::now();
    let c1 = compute_c1_native(x, y, z, k, primes, count_table);
    println!("[TITAN-PERF] native C1 latency: {:?}", t_c1.elapsed());
    a - c1 + c2
}

// ---------------------------------------------------------------------------
// Strike 4 (revised): sieved quotient windows + windowed A-leaves.
// ---------------------------------------------------------------------------
//
// Titan's `SegmentedPiTable` marks windows from a precomputed prime list, so
// windows above the known-primes range cannot use it. These quotient windows
// instead SIEVE [low, high) directly (Walisch-style): only base primes up to
// sqrt(high) are needed, the active bitset stays L1D-resident, and absolute
// counts anchor on the small exact `PiTable`.

/// Quotient segment length in integers (power of two).
/// 16384 integers -> 8192 odd bits (1 KiB) + 128 prefix words (1 KiB).
pub const SEGMENT_Q: u64 = 16384;

/// A sieved, cache-resident window over quotient space [low, high).
///
/// Bitset covers odd integers only; `prefix[i]` holds the prime count of
/// complete 64-odd blocks before block `i`, so queries are O(1).
pub struct QuotientWindow {
    pub low: u64,
    pub high: u64,
    /// Absolute prime count of integers strictly below `low`.
    pub base: u64,
    bits: Vec<u64>,
    prefix: Vec<u64>,
}

impl QuotientWindow {
    /// Sieve `[low, high)` with `base_primes` (must reach `sqrt(high-1)`).
    /// `base_pi_below_low` = pi(low - 1) from an exact table (0 if low == 0).
    pub fn sieve(low: u64, high: u64, base_primes: &[u64], base_pi_below_low: u64) -> Self {
        assert!(high > low, "empty quotient window");
        // Windows tile from odd lows with even stride (cursor starts at 1,
        // SEGMENT_Q is even), so first_odd == low and every integer in
        // [low, high) is either represented or even-and-composite... except 2.
        // v < first_odd can therefore only mean v < low (caller bug -> assert
        // in pi()). Even low would silently miscount a prime 2: forbid it.
        debug_assert!(low % 2 == 1, "window low must be odd, got {}", low);
        // Odd-only indexing: odd n in [low, high) maps to (n - first_odd) / 2.
        let first_odd = low | 1;
        let n_odds = ((high.saturating_sub(first_odd)) / 2 + 1) as usize;
        let n_words = n_odds.div_ceil(64).max(1);
        let mut bits = vec![!0u64; n_words];
        // Mask out odds >= high in the last word.
        let excess_odds = n_words * 64 - n_odds;
        if excess_odds > 0 {
            bits[n_words - 1] &= !0u64 >> excess_odds;
        }
        // Mask out even... (no evens stored) and, if low is even, nothing to do
        // since first_odd > low already excludes it.
        // 1 is not prime: clear it when covered.
        if first_odd == 1 {
            bits[0] &= !1u64;
        }

        let mark_odd_range = |bits: &mut [u64], lo_odd_idx: usize, hi_odd_idx: usize, step_odds: usize| {
            let mut idx = lo_odd_idx;
            while idx < hi_odd_idx {
                bits[idx / 64] &= !(1u64 << (idx % 64));
                idx += step_odds;
            }
        };

        for &p in base_primes {
            if p < 3 {
                continue; // odds only; 2 never stored
            }
            if p.saturating_mul(p) >= high {
                break; // base_primes ascending: no larger p can mark
            }
            // First multiple of p in [low, high): start at max(p*p, ceil(low/p)*p).
            let mut start = p.saturating_mul(p).max(((low + p - 1) / p) * p);
            // Advance to an odd multiple (even multiples are not stored).
            if start % 2 == 0 {
                start += p;
            }
            if start >= high {
                continue;
            }
            // Only odd multiples from here (step 2p); all are >= start hence odd.
            // Convert to odd-indices relative to first_odd. Consecutive odd
            // multiples of p differ by 2p, i.e. p steps in odd-index space.
            let lo_idx = ((start - first_odd) / 2) as usize;
            let step = p as usize;
            mark_odd_range(&mut bits, lo_idx, n_odds, step);
        }

        // Prefix popcounts per 64-odd block.
        let mut prefix = vec![0u64; n_words];
        let mut run = 0u64;
        for (i, w) in bits.iter().enumerate() {
            prefix[i] = run;
            run += w.count_ones() as u64;
        }

        Self { low, high, base: base_pi_below_low, bits, prefix }
    }

    /// Exact pi(v) for v in [low, high). Loud panic outside (never clamp).
    #[inline]
    pub fn pi(&self, v: u64) -> u64 {
        assert!(
            v >= self.low && v < self.high,
            "quotient {} outside window [{}, {})",
            v,
            self.low,
            self.high
        );
        // Count odds in [first_odd_stored, v].
        let first_odd = self.low | 1;
        let mut count = self.base;
        if v >= first_odd {
            let odd_idx = ((v - first_odd) / 2) as usize;
            let word = odd_idx / 64;
            let bit = odd_idx % 64;
            let mask = if bit == 63 { !0u64 } else { (1u64 << (bit + 1)) - 1 };
            count += self.prefix[word] + (self.bits[word] & mask).count_ones() as u64;
        }
        // The odd-only bitset never stores the even prime 2. Windows with
        // low <= 2 (i.e. the first segment) have base = pi(low - 1) = 0 which
        // excludes it, so add it back for v >= 2. Windows with low >= 3
        // already count 2 inside base.
        if v >= 2 && self.low <= 2 {
            count += 1;
        }
        count
    }
}

/// Strike 4: windowed A-leaves across DynamIQ cores (Walisch-style).
///
/// Quotient space `[1, x/x_star^2]` is tiled into `SEGMENT_Q` windows claimed
/// via an `AtomicU64` cursor. Each worker sieves its window thread-locally
/// (L1D-resident) and evaluates the per-segment narrowed b-range
/// (`min_a`/`max_a` ported from `AC_OpenMP`), so no (b, segment) visit is
/// ever empty by construction... (segments whose narrowed range is empty skip
/// after O(1) bound math). Divisions use the shared reciprocal table.
/// Bit-identical to [`compute_a_formula`].
pub fn compute_a_windowed_mt(
    x: u64,
    y: u64,
    primes: &[u64],
    rec: &[FastDiv64],
    count_table: &PiTable,
    num_threads: usize,
) -> i64 {
    use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

    let x13 = icbrt64(x);
    let x_star = get_x_star_gourdon(x, y);
    if x_star < 1 {
        return 0;
    }
    // Full A quotient span: xpq = xp/q <= x/p^2 <= x/x_star^2.
    let qmax = x / x_star.saturating_mul(x_star).max(1);
    if qmax < 1 {
        return 0;
    }
    let has_sentinel = primes.first() == Some(&0);
    let s: &[u64] = if has_sentinel { &primes[1..] } else { primes };
    if s.is_empty() {
        return 0;
    }
    debug_assert!(rec.len() >= s.len(), "reciprocal table shorter than prime slice");
    let r: &[FastDiv64] = rec;

    // Base primes for sieving windows: need p <= sqrt(qmax).
    let sieve_limit = isqrt64(qmax) + 1;
    let base_end = s.partition_point(|&p| p <= sieve_limit);

    let threads = num_threads.clamp(1, 32);
    let cursor = AtomicU64::new(1);
    let slots: Vec<AtomicI64> = (0..threads).map(|_| AtomicI64::new(0)).collect();
    let cursor_ref = &cursor;
    let slots_ref = &slots;

    std::thread::scope(|scope| {
        for tid in 0..threads {
            scope.spawn(move || {
                titan_pool::worker::bind_worker_affinity(tid);
                let mut local: i64 = 0;
                loop {
                    let low = cursor_ref.fetch_add(SEGMENT_Q, Ordering::Relaxed);
                    if low >= qmax {
                        break;
                    }
                    let high = (low + SEGMENT_Q).min(qmax + 1);
                    // Per-segment narrowed b-range (AC_OpenMP min_a/max_a port,
                    // 1-based Walisch -> 0-based Titan: start = count, end = count).
                    let xlow = x / low.max(1);
                    let xhigh = x / high;
                    let sqrt_xlow = isqrt64(xlow);
                    let max_a_end = count_table.pi(sqrt_xlow.min(x13)) as usize;
                    let min_a_start =
                        count_table.pi(x_star.max((xhigh / high.max(1)).min(x13))) as usize;
                    if min_a_start >= max_a_end {
                        continue;
                    }
                    // Sieve this window thread-locally (L1D-resident).
                    let base = if low <= 1 { 0 } else { count_table.pi(low - 1) };
                    let window =
                        QuotientWindow::sieve(low, high, &s[..base_end], base);
                    for b in min_a_start..max_a_end.min(s.len()) {
                        let p = s[b];
                        if p < 2 {
                            continue;
                        }
                        let xp = x / p;
                        let sqrt_xp = isqrt64(xp);
                        let min_2nd = (xhigh / p).min(sqrt_xp);
                        let j0 = count_table.pi(p.max(min_2nd)) as usize;
                        let max_2nd = (xlow / p).min(sqrt_xp);
                        let e1 = count_table.pi((xp / y.max(1)).min(max_2nd)) as usize;
                        let e2 = count_table.pi(max_2nd) as usize;
                        // Weight 1: xpq in window by construction.
                        let mut j = j0.max(b + 1);
                        while j < e1.min(s.len()) {
                            let xpq = r[j].div(xp);
                            debug_assert!(
                                xpq >= window.low && xpq < window.high,
                                "w1 xpq {} outside [{}, {})",
                                xpq,
                                window.low,
                                window.high
                            );
                            local += window.pi(xpq) as i64;
                            j += 1;
                        }
                        // Weight 2 (x2 multiplier).
                        j = j.max(e1.min(s.len()));
                        while j < e2.min(s.len()) {
                            let xpq = r[j].div(xp);
                            debug_assert!(
                                xpq >= window.low && xpq < window.high,
                                "w2 xpq {} outside [{}, {})",
                                xpq,
                                window.low,
                                window.high
                            );
                            local += (window.pi(xpq) as i64) * 2;
                            j += 1;
                        }
                    }
                }
                slots_ref[tid].store(local, Ordering::Relaxed);
            });
        }
    });

    slots.iter().map(|a| a.load(Ordering::Relaxed)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_a_formula_e13() {
        let x = 10_000_000_000_000u64;
        let y = 103_411u64;
        let max_v = x / y;
        let base_primes = generate_base_primes(max_v.min(10_000_000));
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(max_v);
        let a_val = compute_a_formula(x, y, &primes, &pi_table);
        println!("Computed A(1e13) = {}", a_val);
        assert!(a_val > 0, "A(1e13) must be positive");

        let z = 170_628u64;
        let c2_val = compute_c2_formula(x, y, z, &primes, &pi_table);
        println!("Computed C2(1e13) = {}", c2_val);
    }

    /// Strike 3 parity: MT dispatch must be bit-identical to single-threaded.
    #[test]
    fn test_ac_native_mt_parity() {
        for &x in &[1_000_000_000u64, 100_000_000_000, 1_000_000_000_000] {
            let params = titan_core::tuning::resolve_gourdon_params(x);
            let base = generate_base_primes(params.y);
            let mut primes = vec![0u64];
            primes.extend_from_slice(&base);
            let x_star = crate::sigma_l1::get_x_star_gourdon(x, params.y);
            let pi_table =
                PiTable::new(x_star.saturating_mul(x_star).max(params.z) + 30);

            let st = compute_ac_native(x, params.y, params.z, 8, &primes, &pi_table);
            assert!(st > 0, "native AC must be positive at x = {}", x);
            for &t in &[1usize, 2, 8] {
                let mt =
                    compute_ac_native_mt(x, params.y, params.z, 8, &primes, &pi_table, t);
                assert_eq!(mt, st, "MT({}) diverged from ST at x = {}", t, x);
            }
            println!("AC_MT(1e{}) = {} (ST == MT1 == MT2 == MT8)", (x as f64).log10() as u32, st);
        }
    }

    /// Strike 2 parity: clustered C2 must be bit-identical to the scalar
    /// reference across scales (same iteration space, fewer divs/lookups).
    #[test]
    fn test_c2_clustered_parity() {
        for &x in &[1_000_000_000u64, 100_000_000_000, 1_000_000_000_000] {
            let params = titan_core::tuning::resolve_gourdon_params(x);
            let base = generate_base_primes(params.y);
            let mut primes = vec![0u64];
            primes.extend_from_slice(&base);
            // C2 queries satisfy xpq < x / z.
            let x_star = crate::sigma_l1::get_x_star_gourdon(x, params.y);
            let pi_table =
                PiTable::new(x_star.saturating_mul(x_star).max(params.z) + 30);

            let t0 = std::time::Instant::now();
            let scalar = compute_c2_formula(x, params.y, params.z, &primes, &pi_table);
            let t_scalar = t0.elapsed();
            let t0 = std::time::Instant::now();
            let clustered =
                compute_c2_clustered(x, params.y, params.z, &primes, &pi_table);
            let t_clustered = t0.elapsed();
            println!(
                "C2(1e{}) scalar = {} ({:?}), clustered = {} ({:?})",
                (x as f64).log10() as u32,
                scalar,
                t_scalar,
                clustered,
                t_clustered
            );
            assert!(scalar > 0, "C2 scalar must be positive at x = {}", x);
            assert_eq!(
                clustered, scalar,
                "C2 clustered diverged from scalar at x = {}",
                x
            );
        }
    }

    /// Strike 4: reciprocal table exactness over the live AC query domain.
    /// Every (numerator <= x, prime divisor) pair must match hardware division.
    #[test]
    fn test_fastdiv_exactness_ac_domain() {
        use crate::magic_reciprocal::FastDivTable;

        let x = 1_000_000_000_000u64;
        let params = titan_core::tuning::resolve_gourdon_params(x);
        let base = generate_base_primes(params.y);
        let table = FastDivTable::build(&base, x);
        let rec = table.as_slice();
        assert_eq!(rec.len(), base.len());

        // Numerators spanning the whole AC query range [0, x].
        let mut state = 0x9E3779B97F4A7C15u64;
        let mut next_rand = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..20_000 {
            let qi = (next_rand() as usize) % base.len();
            let q = base[qi];
            // Bias toward small-quotient (clustered) and large-quotient regimes.
            let n = match next_rand() % 4 {
                0 => next_rand() % (x + 1),
                1 => next_rand() % params.y,
                2 => x / q.saturating_sub(next_rand() % 1000).max(1),
                _ => x - next_rand() % 1_000_000,
            };
            assert_eq!(rec[qi].div(n.min(x)), n.min(x) / q, "n = {}, q = {}", n, q);
        }
        // Exhaustive edges: n in {0,1,q-1,q,q+1}, max_n boundary.
        for (qi, &q) in base.iter().enumerate().step_by(977) {
            for &n in &[0u64, 1, q - 1, q, q + 1, x - 1, x] {
                assert_eq!(rec[qi].div(n), n / q, "edge n = {}, q = {}", n, q);
            }
        }
    }

    /// Strike 4 parity: FastDiv MT must equal scalar ST at all thread counts.
    #[test]
    fn test_ac_native_mt_fast_parity() {
        for &x in &[1_000_000_000u64, 100_000_000_000, 1_000_000_000_000] {
            let params = titan_core::tuning::resolve_gourdon_params(x);
            let base = generate_base_primes(params.y);
            let mut primes = vec![0u64];
            primes.extend_from_slice(&base);
            let x_star = crate::sigma_l1::get_x_star_gourdon(x, params.y);
            let pi_table = PiTable::new(x_star.saturating_mul(x_star).max(params.z) + 30);

            let st = compute_ac_native(x, params.y, params.z, 8, &primes, &pi_table);
            for &t in &[1usize, 8] {
                let fast =
                    compute_ac_native_mt_fast(x, params.y, params.z, 8, &primes, &pi_table, t);
                assert_eq!(fast, st, "FastDiv MT({}) diverged from ST at x = {}", t, x);
            }
            println!("AC_FAST(1e{}) = {} (ST == MT1 == MT8)", (x as f64).log10() as u32, st);
        }
    }

    /// Strike 4: sieved-window exactness + windowed-A parity vs scalar A.
    #[test]
    fn test_quotient_window_exactness() {
        let base = generate_base_primes(10_000);
        // Window [1, 16385) checked against a ground-truth sieve.
        let w = QuotientWindow::sieve(1, SEGMENT_Q + 1, &base, 0);
        let mut sieve = vec![true; (SEGMENT_Q + 1) as usize];
        sieve[0] = false;
        if (SEGMENT_Q as usize) >= 1 {
            sieve[1] = false;
        }
        for p in 2..=((SEGMENT_Q as f64).sqrt() as usize) {
            if sieve[p] {
                let mut m = p * p;
                while m <= SEGMENT_Q as usize {
                    sieve[m] = false;
                    m += p;
                }
            }
        }
        let mut run = 0u64;
        for v in 1..=(SEGMENT_Q as u64) {
            if sieve[v as usize] {
                run += 1;
            }
            assert_eq!(w.pi(v), run, "window pi({}) wrong", v);
        }
    }

    /// Strike 4: windowed MT A-leaves must equal scalar A at all thread counts.
    #[test]
    fn test_a_windowed_parity() {
        use crate::magic_reciprocal::FastDivTable;

        for &x in &[1_000_000_000u64, 100_000_000_000, 1_000_000_000_000] {
            let params = titan_core::tuning::resolve_gourdon_params(x);
            let base = generate_base_primes(params.y);
            let mut primes = vec![0u64];
            primes.extend_from_slice(&base);
            let x_star = crate::sigma_l1::get_x_star_gourdon(x, params.y);
            let span = x_star.saturating_mul(x_star).max(params.z) + 30;
            let pi_table = PiTable::new(span);
            let stripped: &[u64] = &primes[1..];
            let div_table = FastDivTable::build(stripped, x);

            let scalar = compute_a_formula(x, params.y, &primes, &pi_table);
            assert!(scalar > 0, "scalar A must be positive at x = {}", x);
            for &t in &[1usize, 8] {
                let win = compute_a_windowed_mt(
                    x,
                    params.y,
                    &primes,
                    div_table.as_slice(),
                    &pi_table,
                    t,
                );
                assert_eq!(win, scalar, "windowed MT({}) diverged at x = {}", t, x);
            }
            println!("A_WIN(1e{}) = {} (scalar == MT1 == MT8)", (x as f64).log10() as u32, scalar);
        }
    }
}
