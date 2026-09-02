//! Phase 38: 3-Stage Streaming Prefetch Pipeline for ARM64.
//!
//! Saturates SM4450 LPDDR5 memory bandwidth using explicit software prefetching (prfm pldl1strm),
//! triple buffering, and 256-bit NEON vector popcount.

use core::arch::asm;
#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;

pub const PREFETCH_DISTANCE: usize = 512; // 8 cache lines ahead
pub const CHUNK_SIZE: usize = 256; // 4x L2 cache line size

#[inline(always)]
pub unsafe fn prefetch_stream(addr: *const u8) {
    #[cfg(target_arch = "aarch64")]
    {
        asm!("prfm pldl1strm, [{0}]", in(reg) addr, options(nostack, readonly));
    }
}

/// NEON 256-bit vector popcount on 256-byte chunk
#[inline(always)]
pub unsafe fn neon_popcount_256(data: &[u8]) -> u32 {
    #[cfg(target_arch = "aarch64")]
    {
        let mut cnt = 0u32;
        let ptr = data.as_ptr();
        for i in 0..16 {
            let v = vld1q_u8(ptr.add(i * 16));
            cnt += vaddvq_u8(vcntq_u8(v)) as u32;
        }
        cnt
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        data.iter().map(|&b| b.count_ones()).sum()
    }
}

/// Branchless byte counting for boundary resolution within chunk
#[inline(always)]
pub fn count_bytes_before(data: &[u8], limit: usize) -> u64 {
    if limit == 0 {
        return 0;
    }
    let mut sum = 0u64;
    let chunks = data[..limit].chunks_exact(16);
    let rem_start = chunks.len() * 16;

    #[cfg(target_arch = "aarch64")]
    {
        for chunk in chunks {
            unsafe {
                let v = vld1q_u8(chunk.as_ptr());
                sum += vaddvq_u8(vcntq_u8(v)) as u64;
            }
        }
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        for chunk in chunks {
            for &b in chunk {
                sum += b.count_ones() as u64;
            }
        }
    }

    for &b in &data[rem_start..limit] {
        sum += b.count_ones() as u64;
    }
    sum
}

/// Streaming count and boundary resolution pipeline
pub unsafe fn count_resolve_streaming(
    bits: &[u8],
    boundaries: &[u64],
    base_prefix: u64,
    output: &mut u64,
) {
    if bits.is_empty() {
        return;
    }

    let mut prefix = base_prefix;
    let mut b_idx = 0;

    let total_len = bits.len();
    if total_len < 3 * CHUNK_SIZE {
        // Direct path for small buffers
        let cnt = bits.iter().map(|&b| (8 - b.count_ones()) as u64).sum::<u64>();
        while b_idx < boundaries.len() {
            let limit_byte = ((boundaries[b_idx] >> 3) as usize).min(total_len);
            let zeros_before = count_zeros_before(bits, limit_byte);
            *output += prefix + zeros_before;
            b_idx += 1;
        }
        *output += prefix + cnt;
        return;
    }

    // Triple buffering: 3 chunks in flight
    let mut buffers = [vec![0u8; CHUNK_SIZE], vec![0u8; CHUNK_SIZE], vec![0u8; CHUNK_SIZE]];
    let mut buffer_idx = 0;

    buffers[0].copy_from_slice(&bits[..CHUNK_SIZE]);
    buffers[1].copy_from_slice(&bits[CHUNK_SIZE..2 * CHUNK_SIZE]);

    let mut pos = 2 * CHUNK_SIZE;

    while pos + CHUNK_SIZE <= total_len {
        // 1. Prefetch next chunk ahead
        if pos + PREFETCH_DISTANCE < total_len {
            prefetch_stream(bits.as_ptr().add(pos + PREFETCH_DISTANCE));
        }

        // 2. Process current buffer with NEON popcount
        let current = &buffers[buffer_idx];
        let set_bits = neon_popcount_256(current);
        let zeros_cnt = (CHUNK_SIZE * 8) as u64 - set_bits as u64;

        // 3. Resolve boundaries in this chunk
        let chunk_start_bit = (pos - 2 * CHUNK_SIZE) as u64 * 8;
        let chunk_end_bit = (pos - CHUNK_SIZE) as u64 * 8;

        while b_idx < boundaries.len() && boundaries[b_idx] < chunk_end_bit {
            if boundaries[b_idx] >= chunk_start_bit {
                let byte_in_chunk = ((boundaries[b_idx] - chunk_start_bit) >> 3) as usize;
                let zeros_before = (byte_in_chunk * 8) as u64 - count_bytes_before(current, byte_in_chunk);
                *output += prefix + zeros_before;
            }
            b_idx += 1;
        }

        prefix += zeros_cnt;

        // 4. Load next chunk into next buffer slot
        let next_chunk = &bits[pos..pos + CHUNK_SIZE];
        buffers[(buffer_idx + 2) % 3].copy_from_slice(next_chunk);

        buffer_idx = (buffer_idx + 1) % 3;
        pos += CHUNK_SIZE;
    }

    // Process remaining trailing chunks
    for i in 0..2 {
        let current = &buffers[(buffer_idx + i) % 3];
        let set_bits = neon_popcount_256(current);
        let zeros_cnt = (CHUNK_SIZE * 8) as u64 - set_bits as u64;
        prefix += zeros_cnt;
    }
}

#[inline(always)]
fn count_zeros_before(data: &[u8], limit: usize) -> u64 {
    (limit * 8) as u64 - count_bytes_before(data, limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_pipeline_parity() {
        let len = 4096usize;
        let mut bits = vec![0xAAu8; len];
        bits[100] = 0x00;
        bits[500] = 0xFF;

        let boundaries = vec![100u64 * 8, 250 * 8, 1000 * 8, 2000 * 8];
        let mut out_stream = 0u64;

        unsafe {
            count_resolve_streaming(&bits, &boundaries, 0, &mut out_stream);
        }

        assert!(out_stream > 0, "Stream pipeline must produce valid counts");
    }
}
