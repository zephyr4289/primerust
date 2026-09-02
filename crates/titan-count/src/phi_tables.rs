use titan_core::phi_tiny::phi_tiny;
use crate::pi_table::PiTable;

pub struct PhiTables;

impl PhiTables {
    /// O(1) evaluation of phi(x, a) for a <= 8 using L1-locked closed forms
    #[inline(always)]
    pub fn phi_small(x: u64, a: usize) -> u64 {
        phi_tiny(x, a as u64)
    }

    /// Evaluates phi(x, a) using recursive branch-pruning and PiTable base cases
    pub fn phi_recursive(
        x: u64,
        a: usize,
        primes: &[u64],
        pi_table: &PiTable,
    ) -> i64 {
        if x == 0 {
            return 0;
        }
        if a == 0 {
            return x as i64;
        }
        if a <= 7 {
            return Self::phi_small(x, a) as i64;
        }

        let p_a = primes[a];
        if x < p_a {
            return 1;
        }
        if (p_a as u128) * (p_a as u128) > (x as u128) {
            // phi(x, a) = pi(x) - a + 1 when p_a^2 > x
            if x <= pi_table.max_y {
                return (pi_table.pi(x) as i64) - (a as i64) + 1;
            }
        }

        // Binary tree step: phi(x, a) = phi(x, a - 1) - phi(x / p_a, a - 1)
        let term1 = Self::phi_recursive(x, a - 1, primes, pi_table);
        let term2 = Self::phi_recursive(x / p_a, a - 1, primes, pi_table);
        term1 - term2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_phi_tables_small() {
        assert_eq!(PhiTables::phi_small(100, 1), 50);
        assert_eq!(PhiTables::phi_small(100, 2), 33);
        assert_eq!(PhiTables::phi_small(100, 3), 26);
    }

    #[test]
    fn test_phi_tables_recursive() {
        let base_primes = generate_base_primes(1000);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(1000);
        let val = PhiTables::phi_recursive(1000, 10, &primes, &pi_table);
        assert!(val > 0);
    }
}
