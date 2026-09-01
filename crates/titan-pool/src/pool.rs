//! Atomic Work Pool: the single-cacheline lock-free unit dispatcher.
//!
//! Acts as the implicit thermal feedback controller.

use crate::unit::WorkUnit;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct WorkPool {
    pub units: Vec<WorkUnit>,
    next_idx: AtomicUsize,
}

impl WorkPool {
    pub fn new(units: Vec<WorkUnit>) -> Self {
        Self {
            units,
            next_idx: AtomicUsize::new(0),
        }
    }

    /// Pull the next work unit. Returns None when all units are exhausted.
    #[inline(always)]
    pub fn pull(&self) -> Option<WorkUnit> {
        let idx = self.next_idx.fetch_add(1, Ordering::Relaxed);
        if idx < self.units.len() {
            Some(self.units[idx])
        } else {
            None
        }
    }

    pub fn total_units(&self) -> usize {
        self.units.len()
    }
}
