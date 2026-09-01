//! EratBig: Segmented bucket sieve for primes p > 4S up to sqrt(N).
//!
//! Primes in this tier cross a segment at most once (or zero times).
//! Bucketing schedules them by target segment so non-crossing segments cost zero cycles.
//!
//! Packed 64-bit Entry Format:
//!   - prime: 24 bits (p <= 10^7 < 2^24)
//!   - rel_byte: 17 bits (byte offset in target segment, <= 65536)
//!   - j: 3 bits (residue step index 0..7)
//!   - row: 3 bits (wheel row index 0..7)
//!   - bit: 3 bits (target bit in byte 0..7)
//!   - rem_segs: 14 bits (remaining segment offset for carry list rollovers)
//!   Total = 64 bits (u64).

use titan_core::wheel::{RESIDUES, WHEEL_INC, WHEEL_NEXT};

pub const BLOCK_CAPACITY: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct BucketEntry {
    pub p: u32,
    pub rel_byte: u32,
    pub j: u8,
    pub row: u8,
    pub bit: u8,
    pub rem_segs: u32,
}

impl BucketEntry {
    #[inline(always)]
    pub fn pack(p: u32, rel_byte: u32, j: u8, row: u8, bit: u8, rem_segs: u32) -> Self {
        debug_assert!(j < 8, "j must be 0..8, got {}", j);
        debug_assert!(row < 8, "row must be 0..8, got {}", row);
        debug_assert!(bit < 8, "bit must be 0..8, got {}", bit);
        debug_assert!(p <= 100_000_000, "prime p exceeds domain bound at 10^16/10^18, got {}", p);
        Self { p, rel_byte, j, row, bit, rem_segs }
    }

    #[inline(always)]
    pub fn unpack(self) -> (u32, u32, u8, u8, u8, u32) {
        (self.p, self.rel_byte, self.j, self.row, self.bit, self.rem_segs)
    }
}

pub struct BucketBlock {
    pub entries: [BucketEntry; BLOCK_CAPACITY],
    pub count: usize,
    pub next: Option<usize>,
}

impl BucketBlock {
    pub fn new() -> Self {
        Self {
            entries: [BucketEntry::default(); BLOCK_CAPACITY],
            count: 0,
            next: None,
        }
    }
}

/// Pre-allocated Block Pool to eliminate runtime allocations
pub struct BlockPool {
    pub blocks: Vec<BucketBlock>,
    pub free_head: Option<usize>,
}

impl BlockPool {
    pub fn new(capacity: usize) -> Self {
        let mut blocks = Vec::with_capacity(capacity);
        for i in 0..capacity {
            let mut block = BucketBlock::new();
            if i + 1 < capacity {
                block.next = Some(i + 1);
            }
            blocks.push(block);
        }
        let free_head = if capacity > 0 { Some(0) } else { None };
        Self { blocks, free_head }
    }

    pub fn alloc_block(&mut self) -> usize {
        if let Some(idx) = self.free_head {
            self.free_head = self.blocks[idx].next;
            self.blocks[idx].count = 0;
            self.blocks[idx].next = None;
            idx
        } else {
            let idx = self.blocks.len();
            self.blocks.push(BucketBlock::new());
            idx
        }
    }

    pub fn free_block(&mut self, idx: usize) {
        self.blocks[idx].next = self.free_head;
        self.free_head = Some(idx);
    }
}

/// Ring of W segment buckets with Carry list
pub struct BucketRing {
    pub window_size: usize,
    pub ring_heads: Vec<Option<usize>>,
    pub carry_heads: Option<usize>,
    pub pool: BlockPool,
}

impl BucketRing {
    pub fn new(window_size: usize, pool_capacity: usize) -> Self {
        Self {
            window_size,
            ring_heads: vec![None; window_size],
            carry_heads: None,
            pool: BlockPool::new(pool_capacity),
        }
    }

    pub fn reset(&mut self) {
        for head in &mut self.ring_heads {
            let mut cur = *head;
            while let Some(idx) = cur {
                let next = self.pool.blocks[idx].next;
                self.pool.free_block(idx);
                cur = next;
            }
            *head = None;
        }
        let mut cur = self.carry_heads;
        while let Some(idx) = cur {
            let next = self.pool.blocks[idx].next;
            self.pool.free_block(idx);
            cur = next;
        }
        self.carry_heads = None;
    }

    /// Push an entry into a bucket slot (slot is 0..W-1)
    #[inline(always)]
    pub fn push_ring(&mut self, slot: usize, entry: BucketEntry) {
        let head = self.ring_heads[slot];
        if let Some(h_idx) = head {
            if self.pool.blocks[h_idx].count < BLOCK_CAPACITY {
                let cnt = self.pool.blocks[h_idx].count;
                self.pool.blocks[h_idx].entries[cnt] = entry;
                self.pool.blocks[h_idx].count += 1;
                return;
            }
        }
        let new_idx = self.pool.alloc_block();
        self.pool.blocks[new_idx].entries[0] = entry;
        self.pool.blocks[new_idx].count = 1;
        self.pool.blocks[new_idx].next = head;
        self.ring_heads[slot] = Some(new_idx);
    }

    /// Push an entry into the carry list (for rollovers > current window)
    #[inline(always)]
    pub fn push_carry(&mut self, entry: BucketEntry) {
        let head = self.carry_heads;
        if let Some(h_idx) = head {
            if self.pool.blocks[h_idx].count < BLOCK_CAPACITY {
                let cnt = self.pool.blocks[h_idx].count;
                self.pool.blocks[h_idx].entries[cnt] = entry;
                self.pool.blocks[h_idx].count += 1;
                return;
            }
        }
        let new_idx = self.pool.alloc_block();
        self.pool.blocks[new_idx].entries[0] = entry;
        self.pool.blocks[new_idx].count = 1;
        self.pool.blocks[new_idx].next = head;
        self.carry_heads = Some(new_idx);
    }

    /// Drain bucket for segment `slot` into `segment_buf`, re-pushing next crossings
    pub fn drain_segment(
        &mut self,
        slot: usize,
        segment_buf: &mut [u8],
        seg_size: usize,
    ) {
        let mut cur_block = self.ring_heads[slot];
        self.ring_heads[slot] = None; // Drained

        let w = self.window_size;
        let mut to_ring = Vec::new();
        let mut to_carry = Vec::new();

        while let Some(b_idx) = cur_block {
            let count = self.pool.blocks[b_idx].count;
            for i in 0..count {
                let entry = self.pool.blocks[b_idx].entries[i];
                let (p, mut rel_byte, mut j, row, mut bit, _) = entry.unpack();

                // Cross off all occurrences within the current segment
                while (rel_byte as usize) < seg_size {
                    unsafe {
                        *segment_buf.get_unchecked_mut(rel_byte as usize) &= !(1 << bit);
                    }
                    let inc = WHEEL_INC[j as usize] as u64;
                    let next_j = (j as usize + 1) & 7;
                    let next_bit = WHEEL_NEXT[row as usize][next_j];
                    let res = RESIDUES[bit as usize] as u64;
                    let delta = ((res + (p as u64) * inc) / 30) as u32;

                    rel_byte += delta;
                    j = next_j as u8;
                    bit = next_bit;
                }

                // Prime has now stepped beyond the current segment
                let seg_offset = (rel_byte as usize) / seg_size;
                let target_rel_byte = (rel_byte as u32) % (seg_size as u32);
                let target_global_slot = slot + seg_offset;

                if target_global_slot < w {
                    let next_entry = BucketEntry::pack(p, target_rel_byte, j, row, bit, 0);
                    to_ring.push((target_global_slot, next_entry));
                } else {
                    let rem_segs = (target_global_slot - w) as u32;
                    let next_entry = BucketEntry::pack(p, target_rel_byte, j, row, bit, rem_segs);
                    to_carry.push(next_entry);
                }
            }
            let next_b = self.pool.blocks[b_idx].next;
            self.pool.free_block(b_idx);
            cur_block = next_b;
        }

        for (target_slot, entry) in to_ring {
            self.push_ring(target_slot, entry);
        }
        for entry in to_carry {
            self.push_carry(entry);
        }
    }

    /// Advance window by W: re-slot carry list entries into new window
    pub fn advance_window(&mut self) {
        let mut cur_block = self.carry_heads;
        self.carry_heads = None;

        let w = self.window_size;
        let mut new_carries = Vec::new();
        let mut to_ring = Vec::new();

        while let Some(b_idx) = cur_block {
            let count = self.pool.blocks[b_idx].count;
            for i in 0..count {
                let entry = self.pool.blocks[b_idx].entries[i];
                let (p, rel_byte, j, row, bit, rem_segs) = entry.unpack();

                if (rem_segs as usize) < w {
                    let target_slot = rem_segs as usize;
                    let new_entry = BucketEntry::pack(p, rel_byte, j, row, bit, 0);
                    to_ring.push((target_slot, new_entry));
                } else {
                    let new_rem = rem_segs - (w as u32);
                    let new_entry = BucketEntry::pack(p, rel_byte, j, row, bit, new_rem);
                    new_carries.push(new_entry);
                }
            }
            let next_b = self.pool.blocks[b_idx].next;
            self.pool.free_block(b_idx);
            cur_block = next_b;
        }

        for (slot, entry) in to_ring {
            self.push_ring(slot, entry);
        }
        for entry in new_carries {
            self.push_carry(entry);
        }
    }
}
