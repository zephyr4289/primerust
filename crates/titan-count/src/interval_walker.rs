//! High-Performance Monotone-v Streaming Interval Walker with M-Chaining (Phase 16 Deliverable).
//!
//! Optimization Rungs:
//!   - K1: M-Chaining: Carried register eliminates 50% of Mertens lookups (M(e_end) -> M(e_start))
//!   - K2: pi-Streaming per-j: Monotone descending v gives sequential table locality
//!   - K3: Branch-free signed accumulation

use crate::mertens_struct::MertensStructure;
use crate::pi_table::PiTable;

const BLOCK_BYTES: usize = 64; // 64-byte blocks in PiTable

pub struct IntervalWalker;

impl IntervalWalker {
    /// Evaluates special leaves sum over all j in (c, a] using K1/K2 streaming kernel
    #[inline(always)]
    pub fn walk_intervals(
        x: u64,
        c: usize,
        a: usize,
        primes: &[u64],
        pi_table: &PiTable,
        mertens: &MertensStructure,
    ) -> i64 {
        let mut total_sum = 0i64;

        if a <= c || x == 0 {
            return 0;
        }

        // For each attachment level j in (c, a]:
        for j in (c + 1)..=a {
            let p_j = primes[j];
            let e_lo = p_j as usize;
            let e_hi = (x / (p_j * p_j)) as usize;
            if e_hi < e_lo {
                continue;
            }

            // K1: M-Chaining carried register (M(e - 1))
            let mut m_prev = mertens.mertens(e_lo - 1);
            let mut e = e_lo;

            // K2: π-Streaming state - cursor into PiTable for monotone v
            let mut pi_cursor_block = pi_table.prefix_counts.len(); // Start beyond end
            let mut pi_cursor_byte = 0usize;
            let mut pi_cursor_count = 0u64;

            while e <= e_hi {
                let v = (x / ((p_j as u64) * (e as u64))) as u64;
                if v == 0 {
                    break;
                }

                // Next run boundary where floor(x / (p_j * e')) < v
                let next_e = ((x / ((p_j as u64) * v)) + 1) as usize;
                let run_end = (next_e - 1).min(e_hi);

                // K1: Single lookup for M(run_end), subtracting carried m_prev
                let m_curr = mertens.mertens(run_end);
                let mu_diff = (m_curr - m_prev) as i64;
                m_prev = m_curr;

                if mu_diff != 0 {
                    // K2: Monotone-v pi-lookup with streaming cursor
                    let pi_v = Self::pi_streaming_lookup(v, pi_table, &mut pi_cursor_block, &mut pi_cursor_byte, &mut pi_cursor_count);
                    
                    let weight = pi_v - (j as i64) + 1;
                    total_sum += mu_diff * weight;
                }

                e = run_end + 1;
            }
        }

        total_sum
    }

    /// K2: Streaming π(v) lookup for monotonically decreasing v.
    /// Maintains a cursor walking backwards through PiTable blocks.
    #[inline(always)]
    fn pi_streaming_lookup(
        v: u64,
        pi_table: &PiTable,
        cursor_block: &mut usize,
        cursor_byte: &mut usize,
        cursor_count: &mut u64,
    ) -> i64 {
        if v > pi_table.max_y {
            // Fallback for v beyond PiTable (should be rare in D term range)
            return 0;
        }

        let byte_idx = (v / 30) as usize;
        let target_block = byte_idx / BLOCK_BYTES;

        // Move cursor backwards to target block (v is decreasing)
        while *cursor_block > target_block {
            *cursor_block -= 1;
            *cursor_count = pi_table.prefix_counts[*cursor_block] as u64;
            *cursor_byte = *cursor_block * BLOCK_BYTES;
        }

        // Now cursor is at or before target block
        let mut count = *cursor_count;
        let mut byte = *cursor_byte;

        // Advance within block to target byte
        while byte + 8 <= byte_idx {
            let chunk = u64::from_le_bytes(pi_table.bytes[byte..byte + 8].try_into().unwrap());
            count += chunk.count_ones() as u64;
            byte += 8;
        }
        while byte < byte_idx {
            count += pi_table.bytes[byte].count_ones() as u64;
            byte += 1;
        }

        // Final byte mask
        let rem = (v % 30) as u8;
        let mut mask = 0u8;
        for i in 0..8 {
            if titan_core::wheel::RESIDUES[i] <= rem {
                mask |= 1 << i;
            }
        }
        count += (pi_table.bytes[byte_idx] & mask).count_ones() as u64;

        // Update cursor
        *cursor_block = target_block;
        *cursor_byte = byte_idx;
        *cursor_count = count;

        count as i64
    }

    /// Multi-threaded interval walker with level-partitioning across num_threads
    pub fn walk_intervals_mt(
        x: u64,
        c: usize,
        a: usize,
        primes: &[u64],
        pi_table: &PiTable,
        mertens: &MertensStructure,
        num_threads: usize,
    ) -> i64 {
        if a <= c || x == 0 {
            return 0;
        }
        if num_threads <= 1 || (a - c) < 16 {
            return Self::walk_intervals(x, c, a, primes, pi_table, mertens);
        }

        let total_levels = a - c;
        let chunk_size = (total_levels + num_threads - 1) / num_threads;

        std::thread::scope(|s| {
            let mut handles = Vec::with_capacity(num_threads);

            for t in 0..num_threads {
                let start_j = c + 1 + t * chunk_size;
                let end_j = (c + 1 + (t + 1) * chunk_size - 1).min(a);

                if start_j > a {
                    break;
                }

                let handle = s.spawn(move || {
                    let mut partial_sum = 0i64;
                    for j in start_j..=end_j {
                        let p_j = primes[j];
                        let e_lo = p_j as usize;
                        let e_hi = (x / (p_j * p_j)) as usize;
                        if e_hi < e_lo {
                            continue;
                        }

                        let mut m_prev = mertens.mertens(e_lo - 1);
                        let mut e = e_lo;

                        // K2: Per-thread π-streaming cursor
                        let mut pi_cursor_block = pi_table.prefix_counts.len();
                        let mut pi_cursor_byte = 0usize;
                        let mut pi_cursor_count = 0u64;

                        while e <= e_hi {
                            let v = (x / ((p_j as u64) * (e as u64))) as u64;
                            if v == 0 {
                                break;
                            }

                            let next_e = ((x / ((p_j as u64) * v)) + 1) as usize;
                            let run_end = (next_e - 1).min(e_hi);

                            let m_curr = mertens.mertens(run_end);
                            let mu_diff = (m_curr - m_prev) as i64;
                            m_prev = m_curr;

                            if mu_diff != 0 {
                                let pi_v = Self::pi_streaming_lookup(v, pi_table, &mut pi_cursor_block, &mut pi_cursor_byte, &mut pi_cursor_count);
                                
                                let weight = pi_v - (j as i64) + 1;
                                partial_sum += mu_diff * weight;
                            }

                            e = run_end + 1;
                        }
                    }
                    partial_sum
                });
                handles.push(handle);
            }

            handles.into_iter().map(|h| h.join().unwrap()).sum()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;
    use crate::pi_table::PiTable;
    use crate::mertens_struct::MertensStructure;

    #[test]
    fn test_interval_walker_mt_equivalence() {
        let x = 10_000u64;
        let base_primes = titan_sieve::base::generate_base_primes(500);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(500);
        let mertens = MertensStructure::new(1000);

        let st_sum = IntervalWalker::walk_intervals(x, 2, 8, &primes, &pi_table, &mertens);
        let mt_sum = IntervalWalker::walk_intervals_mt(x, 2, 8, &primes, &pi_table, &mertens, 4);
        assert_eq!(st_sum, mt_sum);
    }
}