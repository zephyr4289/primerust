//! P3: Third partial sifting term P3(x, a, c) via prefetch-friendly pi_table walks.
//!
//! Evaluates:
//!   P3(x, a, c) = sum_{i=a+1}^c sum_{j=i}^{pi(sqrt(x/p_i))} [ pi(x / (p_i * p_j)) - (j - 1) ]

use crate::pi_table::PiTable;
use titan_core::roots::isqrt;

pub fn compute_p3(
    x: u64,
    a: usize,
    c: usize,
    primes: &[u64],
    pi_table: &PiTable,
) -> u64 {
    if a >= c {
        return 0; // P3 vanishes when a >= c (Meissel's condition)
    }

    let mut p3_sum = 0u64;

    for i in (a + 1)..=c {
        let p_i = primes[i];
        let x_div_pi = x / p_i;
        let sqrt_x_div_pi = isqrt(x_div_pi);
        let j_max = pi_table.pi(sqrt_x_div_pi) as usize;

        if j_max < i {
            continue;
        }

        let mut inner_sum = 0u64;
        for j in i..=j_max {
            let p_j = primes[j];
            let y = x_div_pi / p_j;
            let pi_y = pi_table.pi(y);
            let term = pi_y - (j as u64 - 1);
            inner_sum += term;
        }
        p3_sum += inner_sum;
    }

    p3_sum
}
