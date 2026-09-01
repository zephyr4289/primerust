//! P2: Second partial sifting term P2(x, a, b) via O(1) table lookups.
//!
//! Evaluates:
//!   P2(x, a, b) = sum_{i=a+1}^b pi(x / p_i)

use crate::pi_table::PiTable;

pub fn compute_p2(
    x: u64,
    a: usize,
    b: usize,
    primes: &[u64],
    pi_table: &PiTable,
) -> u128 {
    if a >= b {
        return 0;
    }

    let mut p2_total = 0u128;

    for i in (a + 1)..=b {
        let p_i = primes[i];
        let y = x / p_i;
        p2_total += pi_table.pi(y) as u128;
    }

    p2_total
}
