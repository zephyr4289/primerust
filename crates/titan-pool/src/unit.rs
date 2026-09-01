//! Work Unit: 64 KiB boundary-aligned partition of the [0, N] search space.

pub const BASE_UNIT_SPAN: u64 = 30 * 65536; // 1,966,080 numbers

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkUnit {
    pub id: usize,
    pub lo: u64,
    pub hi: u64,
}

/// Partition [0, n] into exactly aligned work units.
///
/// Every unit boundary (except the final end n) is an exact multiple
/// of 1,966,080 numbers, ensuring seamless alignment for both
/// 64 KiB (A76) and 32 KiB (A55) segment buffers.
pub fn generate_work_units(n: u64, target_unit_count: usize) -> Vec<WorkUnit> {
    if n == 0 {
        return Vec::new();
    }
    if n <= BASE_UNIT_SPAN {
        return vec![WorkUnit { id: 0, lo: 0, hi: n }];
    }

    let approx_span = n / (target_unit_count as u64);
    let num_blocks = (approx_span / BASE_UNIT_SPAN).max(1);
    let unit_span = num_blocks * BASE_UNIT_SPAN;

    let mut units = Vec::new();
    let mut lo = 0u64;
    let mut id = 0usize;

    while lo <= n {
        let hi = (lo + unit_span - 1).min(n);
        units.push(WorkUnit { id, lo, hi });
        if hi == n {
            break;
        }
        lo = hi + 1;
        id += 1;
    }

    units
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_partition_cover_exact() {
        let n = 10_000_000_000u64;
        let units = generate_work_units(n, 64);

        assert!(!units.is_empty());
        assert_eq!(units[0].lo, 0);
        assert_eq!(units.last().unwrap().hi, n);

        // Check seamless cover: no gaps, no overlaps
        for w in units.windows(2) {
            assert_eq!(w[0].hi + 1, w[1].lo, "Seam defect between units {} and {}!", w[0].id, w[1].id);
            assert_eq!(w[1].lo % BASE_UNIT_SPAN, 0, "Unit {} lo is not 64K-aligned!", w[1].id);
        }
    }
}
