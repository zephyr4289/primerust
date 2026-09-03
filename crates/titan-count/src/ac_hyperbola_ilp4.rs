//! Cortex-A78 4-Way Software Pipelined AC Leaf Evaluation.
//! Saturates the Out-of-Order execution window by interleaving 4 reciprocal divisions.

use crate::magic_reciprocal::FastDiv64;
use crate::segmented_pi::SegmentedPiTable;

#[inline(always)]
pub fn process_ac_leaves_ilp4(
    x_div_m: u64,
    mut idx: usize,
    p_end_idx: usize,
    reciprocals: &[FastDiv64],
    pi_table: &SegmentedPiTable,
    leaf_acc: &mut i64,
) -> usize {
    while idx + 4 <= p_end_idx {
        let r0 = unsafe { *reciprocals.get_unchecked(idx) };
        let r1 = unsafe { *reciprocals.get_unchecked(idx + 1) };
        let r2 = unsafe { *reciprocals.get_unchecked(idx + 2) };
        let r3 = unsafe { *reciprocals.get_unchecked(idx + 3) };

        // 1. Issue 4 parallel reciprocal multiplications (UMULH pipelined)
        let v0 = r0.div(x_div_m);
        let v1 = r1.div(x_div_m);
        let v2 = r2.div(x_div_m);
        let v3 = r3.div(x_div_m);

        // 2. Resolve popcounts via ARM64 hardware instructions
        let pi_v0 = pi_table.pi(v0);
        let pi_v1 = pi_table.pi(v1);
        let pi_v2 = pi_table.pi(v2);
        let pi_v3 = pi_table.pi(v3);

        let base_pi_p = (idx + 1) as i64;

        *leaf_acc += ((pi_v0 as i64) - base_pi_p + 1)
                   + ((pi_v1 as i64) - (base_pi_p + 1) + 1)
                   + ((pi_v2 as i64) - (base_pi_p + 2) + 1)
                   + ((pi_v3 as i64) - (base_pi_p + 3) + 1);

        idx += 4;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ilp4_basic() {
        let primes = [2, 3, 5, 7, 11, 13, 17, 19];
        let table = SegmentedPiTable::new(0, 100, &primes);
        let mut acc = 0i64;
        let recips = vec![
            FastDiv64::new(2, 100),
            FastDiv64::new(3, 100),
            FastDiv64::new(5, 100),
            FastDiv64::new(7, 100),
        ];
        let next_idx = process_ac_leaves_ilp4(100, 0, 4, &recips, &table, &mut acc);
        assert_eq!(next_idx, 4);
    }
}
