//! Phase 8.3: Asymmetric DynamIQ Handoff Governor V3 (asymmetric_handoff_v3.rs).
//! Guarantees dual-A78 AC execution and zero idle cycles during the physical sieve.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use titan_core::affinity::pin_to_core;
use titan_core::tuning::{GourdonParams, SEGMENT_BYTES};
use crate::magic_reciprocal::FastDiv64;
use crate::segmented_pi::SegmentedPiTable;
use crate::ac_parallel_v2::{compute_ac_range_ilp4, AcWorkQueue};
use crate::b_term::compute_b_decoupled;

pub struct SieveQueue {
    pub current_segment: AtomicU64,
    pub total_segments: u64,
}

impl SieveQueue {
    pub fn new(total_segments: u64) -> Self {
        Self {
            current_segment: AtomicU64::new(0),
            total_segments,
        }
    }

    #[inline(always)]
    pub fn claim(&self, preferred_batch: u64) -> Option<(u64, u64)> {
        let mut curr = self.current_segment.load(Ordering::Relaxed);
        loop {
            if curr >= self.total_segments {
                return None;
            }
            let remaining = self.total_segments - curr;
            // Taper chunk size near the end to prevent A55 stragglers from delaying completion
            let actual_batch = if remaining < 512 {
                (remaining / 8).clamp(1, 8)
            } else {
                preferred_batch
            };

            let end = (curr + actual_batch).min(self.total_segments);
            match self.current_segment.compare_exchange_weak(
                curr,
                end,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some((curr, end)),
                Err(actual) => curr = actual,
            }
        }
    }
}

pub fn execute_asymmetric_gourdon_v3(
    params: &GourdonParams,
    mu: Arc<Vec<i8>>,
    primes: Arc<Vec<u64>>,
    reciprocals: Arc<Vec<FastDiv64>>,
    pi_table: Arc<SegmentedPiTable>,
) -> (i64, i64, i64) {
    let sieve_queue = Arc::new(SieveQueue::new(params.total_segments));
    let ac_queue = Arc::new(AcWorkQueue::new(params.y));

    // 1. Spawn Cortex-A55 Little Workers (Cores 0..=5) -> Exclusively stream D
    let mut little_handles = Vec::with_capacity(6);
    for core_id in 0..=5 {
        let sq = Arc::clone(&sieve_queue);
        let p = Arc::clone(&primes);
        let par = *params;

        little_handles.push(thread::spawn(move || {
            pin_to_core(core_id);
            let mut scratchpad = Box::new([0xFFu8; SEGMENT_BYTES]);
            let mut sum_d = 0i64;

            while let Some((start_seg, end_seg)) = sq.claim(16) {
                for seg_idx in start_seg..end_seg {
                    sum_d += crate::d_worker::sieve_into_scratchpad(
                        seg_idx,
                        &par,
                        &p,
                        scratchpad.as_mut(),
                    );
                }
            }
            sum_d
        }));
    }

    // 2. Spawn Cortex-A78 Big Worker 0 (Core 6) -> B(x, y) + Concurrent AC + Steal D
    let sq_6 = Arc::clone(&sieve_queue);
    let ac_6 = Arc::clone(&ac_queue);
    let p_6 = Arc::clone(&primes);
    let r_6 = Arc::clone(&reciprocals);
    let pi_6 = Arc::clone(&pi_table);
    let mu_6 = Arc::clone(&mu);
    let par_6 = *params;

    let handle_core6 = thread::spawn(move || {
        pin_to_core(6);

        // Core 6 computes B(x, y)
        let b_val = compute_b_decoupled(par_6.x, par_6.y, &p_6, &r_6);

        // Core 6 participates in parallel AC
        let mut ac_val = 0i64;
        while let Some((m_start, m_end)) = ac_6.claim_chunk() {
            ac_val += compute_ac_range_ilp4(
                m_start, m_end, par_6.x, par_6.z, &mu_6, &p_6, &r_6, &pi_6,
            );
        }

        // Core 6 transitions to D-sieve (claims 64-segment batches)
        let mut scratchpad = Box::new([0xFFu8; SEGMENT_BYTES]);
        let mut d_stolen = 0i64;
        while let Some((start_seg, end_seg)) = sq_6.claim(64) {
            for seg_idx in start_seg..end_seg {
                d_stolen += crate::d_worker::sieve_into_scratchpad(
                    seg_idx,
                    &par_6,
                    &p_6,
                    scratchpad.as_mut(),
                );
            }
        }

        (b_val, ac_val, d_stolen)
    });

    // 3. Spawn Cortex-A78 Big Worker 1 (Core 7) -> Concurrent AC immediately + Steal D
    let sq_7 = Arc::clone(&sieve_queue);
    let ac_7 = Arc::clone(&ac_queue);
    let p_7 = Arc::clone(&primes);
    let r_7 = Arc::clone(&reciprocals);
    let pi_7 = Arc::clone(&pi_table);
    let mu_7 = Arc::clone(&mu);
    let par_7 = *params;

    let handle_core7 = thread::spawn(move || {
        pin_to_core(7);

        // Core 7 participates in parallel AC from t = 0
        let mut ac_val = 0i64;
        while let Some((m_start, m_end)) = ac_7.claim_chunk() {
            ac_val += compute_ac_range_ilp4(
                m_start, m_end, par_7.x, par_7.z, &mu_7, &p_7, &r_7, &pi_7,
            );
        }

        // Core 7 transitions to D-sieve (claims 64-segment batches)
        let mut scratchpad = Box::new([0xFFu8; SEGMENT_BYTES]);
        let mut d_stolen = 0i64;
        while let Some((start_seg, end_seg)) = sq_7.claim(64) {
            for seg_idx in start_seg..end_seg {
                d_stolen += crate::d_worker::sieve_into_scratchpad(
                    seg_idx,
                    &par_7,
                    &p_7,
                    scratchpad.as_mut(),
                );
            }
        }

        (ac_val, d_stolen)
    });

    let (b_val, ac_6, d_6) = handle_core6.join().unwrap();
    let (ac_7, d_7) = handle_core7.join().unwrap();

    let mut total_d = d_6 + d_7;
    for h in little_handles {
        total_d += h.join().unwrap();
    }

    (b_val, ac_6 + ac_7, total_d)
}
