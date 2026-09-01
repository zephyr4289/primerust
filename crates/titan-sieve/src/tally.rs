//! Tally: popcount counting over segment buffers.
//!
//! Provides scalar word-wise counting and ARM NEON (vcntq_u8) vector kernel.


/// Count all set bits in data[..bytes_len], with last byte masked up to last_valid_bit.
pub fn count_segment(data: &mut [u8], valid_bytes: usize, last_valid_bit: u8) -> u64 {
    if valid_bytes == 0 {
        return 0;
    }
    // Apply end mask on the last byte
    debug_assert!(last_valid_bit < 8);
    let mask = titan_core::wheel::HIGH_MASK[last_valid_bit as usize];
    data[valid_bytes - 1] &= mask;

    #[cfg(target_arch = "aarch64")]
    unsafe {
        neon_count_bytes(&data[..valid_bytes])
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        scalar_count_bytes(&data[..valid_bytes])
    }
}

/// Scalar word-wise popcount.
pub fn scalar_count_bytes(data: &[u8]) -> u64 {
    let mut count = 0u64;
    let mut chunks = data.chunks_exact(8);
    for chunk in &mut chunks {
        let word = u64::from_ne_bytes(chunk.try_into().unwrap());
        count += word.count_ones() as u64;
    }
    for &b in chunks.remainder() {
        count += b.count_ones() as u64;
    }
    count
}

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn neon_count_bytes(data: &[u8]) -> u64 {
    use std::arch::aarch64::*;
    let len = data.len();
    let mut ptr = data.as_ptr();
    let end = ptr.add(len & !15);
    let mut total = 0u64;

    while ptr < end {
        let v = vld1q_u8(ptr);
        let cnt = vcntq_u8(v);
        total += vaddlvq_u8(cnt) as u64;
        ptr = ptr.add(16);
    }

    let rem = len & 15;
    if rem > 0 {
        let mut tail = [0u8; 16];
        std::ptr::copy_nonoverlapping(ptr, tail.as_mut_ptr(), rem);
        let v = vld1q_u8(tail.as_ptr());
        let cnt = vcntq_u8(v);
        total += vaddlvq_u8(cnt) as u64;
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tally_differential() {
        let mut data = vec![0u8; 1024];
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        for b in data.iter_mut() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *b = (seed >> 32) as u8;
        }

        let scalar = scalar_count_bytes(&data);

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let neon = neon_count_bytes(&data);
            assert_eq!(neon, scalar, "NEON count must exactly match scalar popcount!");
        }

        assert!(scalar > 0);
    }
}
