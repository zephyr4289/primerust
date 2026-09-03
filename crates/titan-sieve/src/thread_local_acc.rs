//! Phase 3.2: Thread-Isolated Accumulation (thread_local_acc.rs).
//!
//! Cache-line aligned accumulator structure preventing false sharing across DynamIQ interconnect.

#[repr(C, align(64))]
pub struct AlignedAccumulator {
    pub value: i64,
    pub _pad: [u8; 56], // Ensure full 64-byte cache line separation
}

impl AlignedAccumulator {
    pub const fn new() -> Self {
        Self {
            value: 0,
            _pad: [0; 56],
        }
    }
}
