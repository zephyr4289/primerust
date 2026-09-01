//! Substrate Census: Measures the 4 Design Constants for the Interval Substrate.
//!
//! Measures:
//!   1. Distinct-v count: distinct floor(x / d) lookups
//!   2. (j,v)-cell count: total constant-v runs in the interval walker
//!   3. mu-span: sum_j len(e-range_j)
//!   4. v-side sharing ratio

use titan_core::roots::{icbrt, isqrt};

fn census_substrate(x: u64) -> (usize, usize, usize, f64) {
    let x_cbrt = icbrt(x);
    let x_sqrt = isqrt(x);

    let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
    let mut primes = Vec::with_capacity(base_primes.len() + 1);
    primes.push(0);
    primes.extend_from_slice(&base_primes);

    let c = match primes[1..].binary_search(&x_cbrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };
    let a = c; // Meissel / LMO a = pi(x^1/3)

    let mut distinct_v = std::collections::HashSet::new();
    let mut total_jv_cells = 0usize;
    let mut total_mu_span = 0usize;

    // For each attachment level j in 7..=a:
    for j in 7..=a {
        let p_j = primes[j];
        let e_lo = p_j;
        let e_hi = x / (p_j * p_j);
        if e_hi < e_lo {
            continue;
        }

        total_mu_span += (e_hi - e_lo + 1) as usize;

        // Partition [e_lo, e_hi] into maximal constant-v runs
        let mut e = e_lo;
        while e <= e_hi {
            let v = x / (p_j * e);
            distinct_v.insert(v);
            total_jv_cells += 1;

            // Next run boundary where floor(x / (p_j * e')) < v
            // i.e., e' = floor(x / (p_j * v)) + 1
            let next_e = if v == 0 {
                e_hi + 1
            } else {
                (x / (p_j * v)) + 1
            };
            e = next_e.max(e + 1);
        }
    }

    let sharing_ratio = if distinct_v.is_empty() {
        1.0
    } else {
        (total_jv_cells as f64) / (distinct_v.len() as f64)
    };

    (distinct_v.len(), total_jv_cells, total_mu_span, sharing_ratio)
}

fn main() {
    println!("=========================================================================================");
    println!("               SUBSTRATE CENSUS: 4 DESIGN CONSTANTS ACROSS SCALES                        ");
    println!("=========================================================================================");
    println!(" Scale | Distinct-v Lookups | (j,v)-Cell Count (Ops) | mu-Span (Elements) | v-Sharing Ratio");
    println!("-----------------------------------------------------------------------------------------");

    let scales: &[u32] = &[10, 11, 12, 13, 14];

    for &pow in scales {
        let x = 10u64.pow(pow);
        let (distinct_v, jv_cells, mu_span, sharing) = census_substrate(x);
        println!(
            " 10^{:<2} | {:>18} | {:>22} | {:>18} | {:>14.2}:1",
            pow, distinct_v, jv_cells, mu_span, sharing
        );
    }
    println!("=========================================================================================");
}
