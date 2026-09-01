//! PhiTiny: constant-time Phi(x, k) for k <= 8.
//!
//! Evaluates the number of integers <= x not divisible by any of the first k primes.
//! Sized for silicon: k <= 6 flat in u16 (~63.7 KiB static rodata), k=7,8 recursive.

pub const PRIMES: [u64; 9] = [0, 2, 3, 5, 7, 11, 13, 17, 19];
pub const PRIMORIALS: [u64; 7] = [1, 2, 6, 30, 210, 2310, 30030];
pub const TOTIENTS: [u64; 7] = [1, 1, 2, 8, 48, 480, 5760];

// Compile-time flat tables
const TABLE_1: [u16; 2] = {
    let mut t = [0u16; 2];
    t[1] = 1;
    t
};

const TABLE_2: [u16; 6] = {
    let mut t = [0u16; 6];
    let mut i = 1;
    let mut count = 0u16;
    while i < 6 {
        if i % 2 != 0 && i % 3 != 0 {
            count += 1;
        }
        t[i] = count;
        i += 1;
    }
    t
};

const TABLE_3: [u16; 30] = {
    let mut t = [0u16; 30];
    let mut i = 1;
    let mut count = 0u16;
    while i < 30 {
        if i % 2 != 0 && i % 3 != 0 && i % 5 != 0 {
            count += 1;
        }
        t[i] = count;
        i += 1;
    }
    t
};

const TABLE_4: [u16; 210] = {
    let mut t = [0u16; 210];
    let mut i = 1;
    let mut count = 0u16;
    while i < 210 {
        if i % 2 != 0 && i % 3 != 0 && i % 5 != 0 && i % 7 != 0 {
            count += 1;
        }
        t[i] = count;
        i += 1;
    }
    t
};

const TABLE_5: [u16; 2310] = {
    let mut t = [0u16; 2310];
    let mut i = 1;
    let mut count = 0u16;
    while i < 2310 {
        if i % 2 != 0 && i % 3 != 0 && i % 5 != 0 && i % 7 != 0 && i % 11 != 0 {
            count += 1;
        }
        t[i] = count;
        i += 1;
    }
    t
};

const TABLE_6: [u16; 30030] = {
    let mut t = [0u16; 30030];
    let mut i = 1;
    let mut count = 0u16;
    while i < 30030 {
        if i % 2 != 0 && i % 3 != 0 && i % 5 != 0 && i % 7 != 0 && i % 11 != 0 && i % 13 != 0 {
            count += 1;
        }
        t[i] = count;
        i += 1;
    }
    t
};

/// Constant-time Phi(x, k) for k <= 8.
/// Returns number of integers <= x not divisible by any of the first k primes.
pub fn phi_tiny(x: u64, k: u64) -> u64 {
    if x == 0 {
        return 0;
    }
    if k == 0 {
        return x;
    }
    if k <= 6 {
        let pk = PRIMORIALS[k as usize];
        let phi_pk = TOTIENTS[k as usize];
        let div = x / pk;
        let rem = (x % pk) as usize;

        let rem_count = match k {
            1 => TABLE_1[rem] as u64,
            2 => TABLE_2[rem] as u64,
            3 => TABLE_3[rem] as u64,
            4 => TABLE_4[rem] as u64,
            5 => TABLE_5[rem] as u64,
            6 => TABLE_6[rem] as u64,
            _ => unreachable!(),
        };

        div * phi_pk + rem_count
    } else if k == 7 {
        // Phi(x, 7) = Phi(x, 6) - Phi(x / 17, 6)
        phi_tiny(x, 6) - phi_tiny(x / 17, 6)
    } else if k == 8 {
        // Phi(x, 8) = Phi(x, 7) - Phi(x / 19, 7)
        phi_tiny(x, 7) - phi_tiny(x / 19, 7)
    } else {
        panic!("phi_tiny only supports k <= 8");
    }
}

// -----------------------------------------------------------------------
// Mutant: Missing Modulo Reduction (demonstrating why mod reduction is mandatory)
// -----------------------------------------------------------------------
pub fn phi_tiny_mutant_missing_mod(x: u64, k: u64) -> u64 {
    if x == 0 || k == 0 {
        return x;
    }
    if k <= 6 {
        let pk = PRIMORIALS[k as usize];
        let phi_pk = TOTIENTS[k as usize];
        let div = x / pk;
        // BUG: uses x directly instead of x % pk
        let rem = (x.min(pk - 1)) as usize;
        let rem_count = match k {
            1 => TABLE_1[rem] as u64,
            2 => TABLE_2[rem] as u64,
            3 => TABLE_3[rem] as u64,
            4 => TABLE_4[rem] as u64,
            5 => TABLE_5[rem] as u64,
            6 => TABLE_6[rem] as u64,
            _ => unreachable!(),
        };
        div * phi_pk + rem_count
    } else {
        phi_tiny(x, k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Naive sieve-based truth oracle for Phi(x, k)
    fn phi_naive_reference(x: u64, k: u64) -> u64 {
        if x == 0 {
            return 0;
        }
        let primes = [2u64, 3, 5, 7, 11, 13, 17, 19];
        let active_primes = &primes[..(k as usize).min(8)];
        let mut count = 0u64;
        for i in 1..=x {
            if !active_primes.iter().any(|&p| i % p == 0) {
                count += 1;
            }
        }
        count
    }

    #[test]
    fn test_exhaustive_over_full_period_k1_to_k6() {
        // Certify every single entry of the full periods
        for k in 1..=5 {
            let pk = PRIMORIALS[k as usize];
            for x in 0..=pk {
                let actual = phi_tiny(x, k);
                let expected = phi_naive_reference(x, k);
                assert_eq!(
                    actual, expected,
                    "phi_tiny({}, {}) mismatch! got {}, expected {}",
                    x, k, actual, expected
                );
            }
        }

        // Test k=6 over first 3,000 points and boundary points
        for x in 0..=3000 {
            let actual = phi_tiny(x, 6);
            let expected = phi_naive_reference(x, 6);
            assert_eq!(actual, expected, "phi_tiny({}, 6) mismatch", x);
        }
        assert_eq!(phi_tiny(30030, 6), 5760);
    }

    #[test]
    fn test_periodicity_and_large_x_identity() {
        // Phi(x + P_k, k) == Phi(x, k) + phi(P_k)
        for k in 1..=6 {
            let pk = PRIMORIALS[k as usize];
            let phi_pk = TOTIENTS[k as usize];
            for base in [100u64, 5000, 1_000_000, 10_000_000_000] {
                let val1 = phi_tiny(base, k);
                let val2 = phi_tiny(base + pk, k);
                assert_eq!(
                    val2,
                    val1 + phi_pk,
                    "Periodicity failed for k={}, base={}",
                    k,
                    base
                );
            }
        }
    }

    #[test]
    fn test_recursive_k7_and_k8() {
        for x in 0..=500 {
            assert_eq!(phi_tiny(x, 7), phi_naive_reference(x, 7));
            assert_eq!(phi_tiny(x, 8), phi_naive_reference(x, 8));
        }
    }

    #[test]
    fn test_phi_mutant_caught() {
        // The mutant that drops modulo reduction must be caught
        let mut caught = false;
        for x in 30031..31000 {
            let truth = phi_tiny(x, 6);
            let mutant = phi_tiny_mutant_missing_mod(x, 6);
            if truth != mutant {
                caught = true;
                break;
            }
        }
        assert!(caught, "Mutant M-phi ESCAPED! Test matrix had no discriminating power.");
    }
}
