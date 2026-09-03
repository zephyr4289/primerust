//! Phase 4.6: Static CSR Bucket Sieve Arena (csr_bucket.rs).
//!
//! To eradicate dynamic heap allocation during the sieving of large primes (p > 65,536),
//! implements a static Compressed Sparse Row (CSR) arena pinned to thread memory.

pub const MAX_BUCKET_ENTRIES: usize = 131_072; // Pinned static capacity per thread

#[derive(Clone, Copy, Debug)]
#[repr(C, align(64))]
pub struct BucketEntry {
    pub prime: u32,
    pub next_offset: u32,
}

#[repr(C, align(64))]
pub struct StaticCsrBucketQueue {
    pub entries: Box<[BucketEntry; MAX_BUCKET_ENTRIES]>,
    pub head: [u32; 1024], // Ring of segment heads
    pub size: usize,
}

impl StaticCsrBucketQueue {
    pub fn new() -> Self {
        let entries = vec![BucketEntry { prime: 0, next_offset: 0 }; MAX_BUCKET_ENTRIES]
            .into_boxed_slice()
            .try_into()
            .unwrap();
        Self {
            entries,
            head: [u32::MAX; 1024],
            size: 0,
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.size = 0;
        self.head.fill(u32::MAX);
    }

    #[inline(always)]
    pub fn push(&mut self, segment_bucket: usize, prime: u32, next_offset: u32) {
        if self.size < MAX_BUCKET_ENTRIES {
            let slot = self.size;
            self.entries[slot] = BucketEntry { prime, next_offset };
            self.head[segment_bucket & 1023] = slot as u32;
            self.size += 1;
        }
    }
}

impl Default for StaticCsrBucketQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_csr_bucket_queue() {
        let mut queue = StaticCsrBucketQueue::new();
        assert_eq!(queue.size, 0);
        queue.push(0, 65537, 100);
        assert_eq!(queue.size, 1);
        assert_eq!(queue.head[0], 0);
        assert_eq!(queue.entries[0].prime, 65537);
        queue.reset();
        assert_eq!(queue.size, 0);
        assert_eq!(queue.head[0], u32::MAX);
    }
}
