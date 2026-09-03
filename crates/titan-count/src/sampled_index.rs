//! Phase 6.1: Two-Tier Sampled Prime Index (sampled_index.rs).
//!
//! Eliminates 26-step DRAM binary search stalls across 203 MB by using
//! a 49.5 KiB cache-locked sample table (Tier 1) followed by a bounded
//! search in a single 16 KiB memory page (Tier 2) with hardware prefetch.

pub const SAMPLE_STRIDE_BITS: usize = 12;
pub const SAMPLE_STRIDE: usize = 1 << SAMPLE_STRIDE_BITS; // 4096 primes per block

#[repr(C, align(64))]
#[derive(Clone, Debug)]
pub struct SampledPrimeIndex {
    /// 49.5 KiB table of prime samples: sample[k] = primes[k * 4096]
    samples: Vec<u32>,
    total_primes: usize,
    max_prime: u64,
}

impl SampledPrimeIndex {
    pub fn build(primes: &[u64]) -> Self {
        if primes.is_empty() {
            return Self {
                samples: Vec::new(),
                total_primes: 0,
                max_prime: 0,
            };
        }

        let num_samples = (primes.len() + SAMPLE_STRIDE - 1) / SAMPLE_STRIDE;
        let mut samples = Vec::with_capacity(num_samples);

        for i in (0..primes.len()).step_by(SAMPLE_STRIDE) {
            unsafe {
                samples.push(*primes.get_unchecked(i) as u32);
            }
        }

        Self {
            samples,
            total_primes: primes.len(),
            max_prime: *primes.last().unwrap(),
        }
    }

    pub fn build_u32(primes: &[u32]) -> Self {
        if primes.is_empty() {
            return Self {
                samples: Vec::new(),
                total_primes: 0,
                max_prime: 0,
            };
        }

        let num_samples = (primes.len() + SAMPLE_STRIDE - 1) / SAMPLE_STRIDE;
        let mut samples = Vec::with_capacity(num_samples);

        for i in (0..primes.len()).step_by(SAMPLE_STRIDE) {
            unsafe {
                samples.push(*primes.get_unchecked(i));
            }
        }

        Self {
            samples,
            total_primes: primes.len(),
            max_prime: *primes.last().unwrap() as u64,
        }
    }

    /// Evaluates pi(v) for any v <= sqrt(x) in bounded cycles:
    /// 1. Binary search inside 49.5 KiB L1D/L2-resident table (14 steps, 0 DRAM misses)
    /// 2. Local bounded binary search inside single 16 KiB contiguous page (12 steps)
    #[inline(always)]
    pub fn pi(&self, primes: &[u64], v: u64) -> u64 {
        if v < 2 {
            return 0;
        }
        if v >= self.max_prime {
            return self.total_primes as u64;
        }

        let v_u32 = v as u32;

        // Tier 1: Search the 49.5 KiB cache-locked sample table
        // Slices fit 100% in Cortex-A78 L1D (64 KiB) or Cortex-A55 L2 (256 KiB)
        let sample_idx = self.samples.partition_point(|&sp| sp <= v_u32);

        // Sub-slice window boundaries
        let low_idx = if sample_idx == 0 { 0 } else { (sample_idx - 1) << SAMPLE_STRIDE_BITS };
        let high_idx = (sample_idx << SAMPLE_STRIDE_BITS).min(self.total_primes);

        // Tier 2: Bounded search within the single 16 KiB window
        unsafe {
            let window = primes.get_unchecked(low_idx..high_idx);

            // Prefetch the midpoint of the 16 KiB window to warm the L1 cacheline
            #[cfg(target_arch = "aarch64")]
            {
                let mid_ptr = window.as_ptr().add(window.len() >> 1);
                std::arch::asm!("prfm pldl1keep, [{}]", in(reg) mid_ptr, options(nostack, preserves_flags));
            }

            let local_offset = window.partition_point(|&p| p <= v);
            (low_idx + local_offset) as u64
        }
    }

    #[inline(always)]
    pub fn pi_u32(&self, primes: &[u32], v: u64) -> u64 {
        if v < 2 {
            return 0;
        }
        if v >= self.max_prime {
            return self.total_primes as u64;
        }

        let v_u32 = v as u32;

        let sample_idx = self.samples.partition_point(|&sp| sp <= v_u32);

        let low_idx = if sample_idx == 0 { 0 } else { (sample_idx - 1) << SAMPLE_STRIDE_BITS };
        let high_idx = (sample_idx << SAMPLE_STRIDE_BITS).min(self.total_primes);

        unsafe {
            let window = primes.get_unchecked(low_idx..high_idx);

            #[cfg(target_arch = "aarch64")]
            {
                let mid_ptr = window.as_ptr().add(window.len() >> 1);
                std::arch::asm!("prfm pldl1keep, [{}]", in(reg) mid_ptr, options(nostack, preserves_flags));
            }

            let local_offset = window.partition_point(|&p| p <= v_u32);
            (low_idx + local_offset) as u64
        }
    }

    #[inline(always)]
    pub fn total_primes(&self) -> usize {
        self.total_primes
    }

    #[inline(always)]
    pub fn table_bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<u32>()
    }
}
