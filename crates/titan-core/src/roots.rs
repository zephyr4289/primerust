//! Integer Roots: exact, guarded, total over all of u64.
//!
//! Evaluates floor(x^(1/k)) such that r^k <= x < (r+1)^k.
//! All power comparisons evaluate in u128 to prevent overflow at u64::MAX.

/// Integer square root: floor(sqrt(x)).
/// Invariant: r^2 <= x < (r + 1)^2 in u128.
#[inline]
pub fn isqrt(x: u64) -> u64 {
    if x < 2 {
        return x;
    }
    // Float seed
    let mut r = (x as f64).sqrt() as u64;
    let xu = x as u128;

    // Two-sided correction guarded in u128
    while (r as u128) * (r as u128) > xu {
        r -= 1;
    }
    while ((r + 1) as u128) * ((r + 1) as u128) <= xu {
        r += 1;
    }
    r
}

/// Integer cube root: floor(x^(1/3)).
/// Invariant: r^3 <= x < (r + 1)^3 in u128.
#[inline]
pub fn icbrt(x: u64) -> u64 {
    if x < 2 {
        return x;
    }
    let mut r = (x as f64).cbrt() as u64;
    let xu = x as u128;

    let cube = |v: u64| -> u128 {
        let vu = v as u128;
        vu * vu * vu
    };

    while cube(r) > xu {
        r -= 1;
    }
    while cube(r + 1) <= xu {
        r += 1;
    }
    r
}

/// Integer 4th root: floor(x^(1/4)).
/// Invariant: r^4 <= x < (r + 1)^4 in u128.
#[inline]
pub fn iroot4(x: u64) -> u64 {
    // Exactly floor(sqrt(floor(sqrt(x))))
    isqrt(isqrt(x))
}

/// Integer k-th root: floor(x^(1/k)) for k in 2..=63.
/// Invariant: r^k <= x < (r + 1)^k in u128.
pub fn iroot(x: u64, k: u32) -> u64 {
    assert!((2..=63).contains(&k), "k must be in 2..=63");
    if x < 2 {
        return x;
    }
    if k == 2 {
        return isqrt(x);
    }
    if k == 3 {
        return icbrt(x);
    }
    if k == 4 {
        return iroot4(x);
    }

    let xu = x as u128;

    // Helper: pow in u128 with saturation
    let pow_u128 = |base: u64, exp: u32| -> u128 {
        let mut res = 1u128;
        let b = base as u128;
        for _ in 0..exp {
            match res.checked_mul(b) {
                Some(p) => res = p,
                None => return u128::MAX,
            }
        }
        res
    };

    // Float seed
    let mut r = (x as f64).powf(1.0 / k as f64) as u64;
    if r == 0 {
        r = 1;
    }

    while pow_u128(r, k) > xu {
        r -= 1;
    }
    while pow_u128(r + 1, k) <= xu {
        r += 1;
    }
    r
}

// -----------------------------------------------------------------------
// Mutant: Uncorrected Float Seed (demonstrating why correction is mandatory)
// -----------------------------------------------------------------------
#[inline(never)]
pub fn isqrt_mutant_uncorrected(x: u64) -> u64 {
    (x as f64).sqrt() as u64
}

#[inline(never)]
pub fn icbrt_mutant_uncorrected(x: u64) -> u64 {
    (x as f64).cbrt() as u64
}

#[inline(never)]
pub fn iroot4_mutant_uncorrected(x: u64) -> u64 {
    (x as f64).powf(0.25) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_identities() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(icbrt(0), 0);
        assert_eq!(icbrt(1), 1);
        assert_eq!(iroot4(0), 0);
        assert_eq!(iroot4(1), 1);

        // u64::MAX boundary
        assert_eq!(isqrt(u64::MAX), 4_294_967_295);
        assert_eq!(icbrt(u64::MAX), 2_642_245);
        assert_eq!(iroot4(u64::MAX), 65_535);

        // Verification in u128
        let r = isqrt(u64::MAX);
        assert!((r as u128) * (r as u128) <= u64::MAX as u128);
        assert!(((r + 1) as u128) * ((r + 1) as u128) > u64::MAX as u128);
    }

    #[test]
    fn test_isqrt_boundary_exhaustive() {
        // r <= 2^18 boundary sweep
        for r in 1u64..=(1 << 18) {
            let sq = r * r;
            // test r^2 - 1
            if sq > 0 {
                let x = sq - 1;
                let root = isqrt(x);
                assert_eq!(root, r - 1, "isqrt({}) failed", x);
            }
            // test r^2
            assert_eq!(isqrt(sq), r, "isqrt({}) failed", sq);
            // test r^2 + 1
            if sq < u64::MAX {
                let x = sq + 1;
                let root = isqrt(x);
                assert_eq!(root, r, "isqrt({}) failed", x);
            }
        }
    }

    #[test]
    fn test_iroot4_exhaustive() {
        // r <= 65535 covers the entire u64 domain!
        for r in 1u64..=65535 {
            let ru = r as u128;
            let p4 = ru * ru * ru * ru;
            if p4 > 0 && p4 - 1 <= u64::MAX as u128 {
                let x = (p4 - 1) as u64;
                assert_eq!(iroot4(x), r - 1, "iroot4({}) failed", x);
            }
            if p4 <= u64::MAX as u128 {
                let x = p4 as u64;
                assert_eq!(iroot4(x), r, "iroot4({}) failed", x);
            }
            if p4 + 1 <= u64::MAX as u128 {
                let x = (p4 + 1) as u64;
                assert_eq!(iroot4(x), r, "iroot4({}) failed", x);
            }
        }
    }

    #[test]
    fn test_high_range_lcg_squares() {
        // Sample near 2^32 with fixed LCG
        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        for _ in 0..100_000 {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let r = isqrt(x);
            let ru = r as u128;
            assert!(ru * ru <= x as u128);
            assert!((ru + 1) * (ru + 1) > x as u128);
        }
    }

    #[test]
    fn test_mutant_m_root_caught() {
        // The uncorrected float seed must be caught by precision loss above 2^52
        let mut caught = false;
        // Test near 2^53 (where float precision loses exactness)
        for r in (1u64 << 27)..(1u64 << 27) + 500_000 {
            let sq = r * r;
            if sq > 0 {
                let x = sq - 1;
                let mutant_val = isqrt_mutant_uncorrected(x);
                let true_val = isqrt(x);
                if mutant_val != true_val {
                    caught = true;
                    break;
                }
            }
        }
        assert!(caught, "Mutant M-root ESCAPED! Test matrix had no discriminating power.");
    }
}
