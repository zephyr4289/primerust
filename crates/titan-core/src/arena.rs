//! Phase 3.4 / Phase 4.4: Zero-Allocation Cache-Aligned Arena (arena.rs).
//!
//! Heterogeneous cache-aligned memory workspace pinned per thread.
//! Implements 2:1 power-of-two L1D geometry for Cortex-A78 (32 KiB) vs Cortex-A55 (16 KiB).

pub const CACHE_LINE: usize = 64;
pub const QUANTUM_SPAN: u64 = 262_144; // Integers per 16 KiB odd-bit segment

pub const WORDS_LITTLE: usize = 2048;  // 16 KiB
pub const PREFIX_LITTLE: usize = 512;  // 2 KiB

pub const WORDS_BIG: usize = 4096;     // 32 KiB
pub const PREFIX_BIG: usize = 1024;    // 4 KiB

/// Cache-line padded container (forces 64-byte alignment and padding).
#[repr(align(64))]
pub struct Padded<T>(pub T);

/// Static, zero-allocation memory workspace pinned per thread.
#[repr(C, align(64))]
pub struct ThreadMemoryArena<const SEG_WORDS: usize, const PREFIX_LEN: usize> {
    pub segment_buf: [u64; SEG_WORDS],
    pub prefix_buf: [u32; PREFIX_LEN],
    pub leaf_drain: [u32; 1024], // Reusable L1D leaf scratchpad
}

impl<const SEG_WORDS: usize, const PREFIX_LEN: usize> ThreadMemoryArena<SEG_WORDS, PREFIX_LEN> {
    pub const fn new() -> Self {
        Self {
            segment_buf: [0u64; SEG_WORDS],
            prefix_buf: [0u32; PREFIX_LEN],
            leaf_drain: [0u32; 1024],
        }
    }

    #[inline(always)]
    pub fn reset_segment(&mut self) {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            let ptr = self.segment_buf.as_mut_ptr() as *mut u8;
            let zero = core::arch::aarch64::vdupq_n_u8(0);
            let bytes = SEG_WORDS * 8;
            for i in (0..bytes).step_by(64) {
                core::arch::aarch64::vst1q_u8(ptr.add(i), zero);
                core::arch::aarch64::vst1q_u8(ptr.add(i + 16), zero);
                core::arch::aarch64::vst1q_u8(ptr.add(i + 32), zero);
                core::arch::aarch64::vst1q_u8(ptr.add(i + 48), zero);
            }
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.segment_buf.fill(0);
        }
    }
}

#[repr(C, align(64))]
pub struct LittleCoreArena {
    pub segment: [u64; WORDS_LITTLE],
    pub prefix: [u32; PREFIX_LITTLE],
}

impl LittleCoreArena {
    pub const fn new() -> Self {
        Self {
            segment: [0u64; WORDS_LITTLE],
            prefix: [0u32; PREFIX_LITTLE],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            let ptr = self.segment.as_mut_ptr() as *mut u8;
            let zero = std::arch::aarch64::vdupq_n_u8(0);
            for i in (0..16384).step_by(64) {
                std::arch::aarch64::vst1q_u8(ptr.add(i), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 16), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 32), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 48), zero);
            }
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.segment.fill(0);
        }
    }
}

#[repr(C, align(64))]
pub struct BigCoreArena {
    pub segment: [u64; WORDS_BIG],
    pub prefix: [u32; PREFIX_BIG],
}

impl BigCoreArena {
    pub const fn new() -> Self {
        Self {
            segment: [0u64; WORDS_BIG],
            prefix: [0u32; PREFIX_BIG],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            let ptr = self.segment.as_mut_ptr() as *mut u8;
            let zero = std::arch::aarch64::vdupq_n_u8(0);
            for i in (0..32768).step_by(64) {
                std::arch::aarch64::vst1q_u8(ptr.add(i), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 16), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 32), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 48), zero);
            }
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.segment.fill(0);
        }
    }
}
