//! Zero-Allocation Tripwire: counting global allocator.
//!
//! Enforces zero allocation in steady state across titan-core.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct CountingAllocator {
    alloc_count: AtomicU64,
    bytes_count: AtomicU64,
}

impl CountingAllocator {
    pub const fn new() -> Self {
        Self {
            alloc_count: AtomicU64::new(0),
            bytes_count: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.alloc_count.store(0, Ordering::SeqCst);
        self.bytes_count.store(0, Ordering::SeqCst);
    }

    pub fn alloc_count(&self) -> u64 {
        self.alloc_count.load(Ordering::SeqCst)
    }

    pub fn bytes_count(&self) -> u64 {
        self.bytes_count.load(Ordering::SeqCst)
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_count.fetch_add(1, Ordering::SeqCst);
        self.bytes_count.fetch_add(layout.size() as u64, Ordering::SeqCst);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
}
