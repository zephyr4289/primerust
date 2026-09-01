//! Mertens Structure (R3b Deliverable).
//!
//! Provides fast interval queries:
//!   M(hi) - M(lo - 1) = sum_{d = lo}^{hi} mu(d)
//! using checkpointed segment boundary sums and local segment prefix arrays.

use crate::mu_sieve::MertensTable;

pub struct MertensStructure {
    pub max_domain: usize,
    table: MertensTable,
}

impl MertensStructure {
    pub fn new(max_domain: usize) -> Self {
        Self {
            max_domain,
            table: MertensTable::new(max_domain),
        }
    }

    /// Evaluates M(u) = sum_{d <= u} mu(d)
    #[inline(always)]
    pub fn mertens(&self, u: usize) -> i32 {
        if u <= self.max_domain {
            self.table.mertens(u)
        } else {
            panic!("Mertens query {} exceeds max_domain {}", u, self.max_domain);
        }
    }

    /// Evaluates interval sum sum_{d = lo}^{hi} mu(d) in O(1)
    #[inline(always)]
    pub fn range_sum(&self, lo: usize, hi: usize) -> i64 {
        if hi < lo || hi == 0 {
            return 0;
        }
        let m_hi = self.mertens(hi) as i64;
        let m_lo = if lo <= 1 { 0 } else { self.mertens(lo - 1) as i64 };
        m_hi - m_lo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mertens_structure_anchors() {
        let m = MertensStructure::new(100_000);
        assert_eq!(m.mertens(1_000), 2);
        assert_eq!(m.mertens(10_000), -23);
        assert_eq!(m.mertens(100_000), -48);

        // Range sum [10, 20]: sum_{d=10}^20 mu(d)
        let sum_10_20 = m.range_sum(10, 20);
        assert_eq!(sum_10_20, (m.mertens(20) - m.mertens(9)) as i64);
    }
}
