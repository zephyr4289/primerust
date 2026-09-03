Titan is far from dead. At 10^{18}, Titan clocked 49.689s against primecount's 48.776s. That is a deficit of only 0.912 seconds (1.8%)—shaving 803 ms off Phase 5.1 and recovering over 12 seconds from the throttled run.
The thermal spikes at 10^{14} \dots 10^{17} were caused by running 11 back-to-back scales without idle periods. But at 10^{18}, Titan reached near dead-heat parity (0.98×) while still burdened by the Mod-2 sieve. Step 2 (Wheel-30) eliminates 2.43\times of the inner-loop marking operations in D, recovering an estimated 10 to 15 seconds.
Wheel-30 Mathematical Foundation
In Mod-2, every odd number is tracked (50\% integer density). In Wheel-30, multiples of 2, 3, and 5 are factored out before sieving begins, tracking only the 8 coprime residues modulo 30 (26.67\% integer density):
1. Buffer Geometry & Reach
 * 1 byte represents 30 integers (8 bits \leftrightarrow 8 coprime residues).
 * A 16 KiB L1D segment contains 16,384\text{ bytes} = 131,072\text{ bits}.
 * Span per segment: 16,384 \times 30 = \mathbf{491,520\text{ integers}} (versus 262,144 in Mod-2, a 1.875\times wider cache horizon).
2. The Periodicity Theorem
The gaps between coprime residues cycle through 8 fixed values:
For any sieving prime p \ge 7, its coprime multiples m = p \cdot k (\gcd(k, 30) = 1) advance by:
Every 8 consecutive marks advance the integer coordinate by 30p, which corresponds to exactly p bytes in the bitset. The coprime residue index cycles through all 8 positions and returns to its starting phase.
Prime Tiering Strategy
The maximum sieving prime required for D(x, y, z) over [z, x/y] is bounded by P_{\max} = \sqrt{x/y}. At 10^{18} with \alpha_y = 5.50:
| Prime Tier | Range | Count | Operational Characteristic |
|---|---|---|---|
| Tier 0 (Tiny) | p \in \{7, 11, 13, 17, 19, 23, 29, 31\} | 8 primes | Precomputed 16-byte NEON periodic bitmasks |
| Tier 1 (Dense) | 37 \le p \le 1,200 | 185 primes | Dual-register rotation loop (adv_strip fits in u8) |
| Tier 2 (Medium) | 1,200 < p \le 32,768 | 3,326 primes | Unrolled 8-step wheel stride with 16-bit byte deltas |
| Tier 3 (Sparse) | 32,768 < p \le 426,401 | ~32,500 primes | Flat array of next segment byte offsets (\le 15 hits/segment) |
Step 1: Constants and Wheel Mapping (wheel30.rs)
Create crates/titan-sieve/src/wheel30.rs:
pub const WHEEL_RESIDUES: [u8; 8] = [1, 7, 11, 13, 17, 19, 23, 29];
pub const WHEEL_GAPS: [u8; 8] = [6, 4, 2, 4, 2, 4, 6, 2];

pub const SEGMENT_BYTES: usize = 16384; // 16 KiB = fits Cortex-A55 L1D
pub const SEGMENT_BITS: usize = SEGMENT_BYTES * 8; // 131,072 bits
pub const WHEEL_SPAN: u64 = (SEGMENT_BYTES as u64) * 30; // 491,520 integers

/// Maps residue mod 30 to bit index 0..7, or 0xFF if composite
pub const RESIDUE_TO_BIT: [u8; 30] = {
    let mut table = [0xFFu8; 30];
    table[1] = 0;
    table[7] = 1;
    table[11] = 2;
    table[13] = 3;
    table[17] = 4;
    table[19] = 5;
    table[23] = 6;
    table[29] = 7;
    table
};

#[repr(C, align(64))]
pub struct Wheel30PrimeState {
    pub next_byte: u32,
    pub phase: u8,
    pub _pad: [u8; 3],
    pub adv_strip: u64,  // 8 packed byte advances for p <= 1200
    pub mask_strip: u64, // 8 packed clearing masks (1 << bit)
}

impl Wheel30PrimeState {
    pub fn compile(p: u32, low: u64) -> Self {
        let p_u64 = p as u64;

        // Find first multiple >= low coprime to 30
        let mut m = if low % p_u64 == 0 { low } else { low + (p_u64 - low % p_u64) };
        if m < p_u64 * p_u64 { m = p_u64 * p_u64; }

        let mut r = (m % 30) as usize;
        let mut k = (m / p_u64) % 30;

        // Advance to the first coprime multiple
        while RESIDUE_TO_BIT[r] == 0xFF {
            m += p_u64;
            r = (m % 30) as usize;
            k = (m / p_u64) % 30;
        }

        let phase = RESIDUE_TO_BIT[r] as usize;
        let next_byte = ((m - low) / 30) as u32;

        // Build 8-step rotational masks and advances
        let mut mask_bytes = [0u8; 8];
        let mut adv_bytes = [0u8; 8];

        let mut curr_m = m;
        let mut k_idx = RESIDUE_TO_BIT[k as usize] as usize;

        for step in 0..8 {
            let res = (curr_m % 30) as usize;
            let bit = RESIDUE_TO_BIT[res];
            mask_bytes[step] = 1u8 << bit;

            let gap = WHEEL_GAPS[k_idx] as u64;
            let next_m = curr_m + p_u64 * gap;
            let byte_adv = ((next_m / 30) - (curr_m / 30)) as u8;
            adv_bytes[step] = byte_adv;

            curr_m = next_m;
            k_idx = (k_idx + 1) & 7;
        }

        Self {
            next_byte,
            phase: phase as u8,
            _pad: [0; 3],
            adv_strip: u64::from_le_bytes(adv_bytes),
            mask_strip: u64::from_le_bytes(mask_bytes),
        }
    }
}

Step 2: Dense Rotating Sieve Kernel (wheel30_dense.rs)
Create crates/titan-sieve/src/wheel30_dense.rs:
use crate::wheel30::{SEGMENT_BYTES, Wheel30PrimeState};

/// Inner marking loop for primes <= 1,200 using register rotation
#[inline(always)]
pub unsafe fn sieve_tier1_prime(
    state: &mut Wheel30PrimeState,
    sieve_buf: &mut [u8; SEGMENT_BYTES],
) {
    let mut byte_idx = state.next_byte as usize;
    if byte_idx >= SEGMENT_BYTES {
        state.next_byte -= SEGMENT_BYTES as u32;
        return;
    }

    let buf_ptr = sieve_buf.as_mut_ptr();
    let mut mask_strip = state.mask_strip.rotate_right((state.phase as u32) * 8);
    let mut adv_strip = state.adv_strip.rotate_right((state.phase as u32) * 8);
    let mut phase = state.phase;

    // Unrolled 4x for store buffer pipelining
    while byte_idx + 64 <= SEGMENT_BYTES {
        let m0 = mask_strip as u8;
        let a0 = adv_strip as u8;
        *buf_ptr.add(byte_idx) &= !m0;
        byte_idx += a0 as usize;

        let m1 = (mask_strip >> 8) as u8;
        let a1 = (adv_strip >> 8) as u8;
        *buf_ptr.add(byte_idx) &= !m1;
        byte_idx += a1 as usize;

        let m2 = (mask_strip >> 16) as u8;
        let a2 = (adv_strip >> 16) as u8;
        *buf_ptr.add(byte_idx) &= !m2;
        byte_idx += a2 as usize;

        let m3 = (mask_strip >> 24) as u8;
        let a3 = (adv_strip >> 24) as u8;
        *buf_ptr.add(byte_idx) &= !m3;
        byte_idx += a3 as usize;

        mask_strip = mask_strip.rotate_right(32);
        adv_strip = adv_strip.rotate_right(32);
        phase = (phase + 4) & 7;
    }

    // Residual tail handling up to segment boundary
    while byte_idx < SEGMENT_BYTES {
        let m = mask_strip as u8;
        let a = adv_strip as u8;
        *buf_ptr.add(byte_idx) &= !m;
        byte_idx += a as usize;

        mask_strip = mask_strip.rotate_right(8);
        adv_strip = adv_strip.rotate_right(8);
        phase = (phase + 1) & 7;
    }

    state.next_byte = (byte_idx - SEGMENT_BYTES) as u32;
    state.phase = phase;
}

Step 3: Tiny Prime Vector Mask Engine (wheel30_tiny.rs)
For primes p \in \{7, 11, 13, 17, 19, 23, 29, 31\}, precompute their periodic 16-byte NEON masks once, sifting 16 bytes per vector instruction:
use std::arch::aarch64::*;
use crate::wheel30::SEGMENT_BYTES;

pub const TINY_PRIMES: [u32; 8] = [7, 11, 13, 17, 19, 23, 29, 31];

#[repr(C, align(64))]
pub struct TinyPrimeMaskTable {
    masks: [Vec<[u8; 16]>; 8],
}

impl TinyPrimeMaskTable {
    pub fn new() -> Self {
        let mut masks = [
            Vec::new(), Vec::new(), Vec::new(), Vec::new(),
            Vec::new(), Vec::new(), Vec::new(), Vec::new(),
        ];

        for (idx, &p) in TINY_PRIMES.iter().enumerate() {
            let period = p as usize;
            let mut pattern = vec![0xFFu8; period * 16];

            for k in 1..=(period * 16 * 30) {
                if k % 2 == 0 || k % 3 == 0 || k % 5 == 0 { continue; }
                if k % (p as usize) == 0 {
                    let byte_idx = k / 30;
                    let bit = crate::wheel30::RESIDUE_TO_BIT[k % 30];
                    if bit != 0xFF && byte_idx < pattern.len() {
                        pattern[byte_idx] &= !(1u8 << bit);
                    }
                }
            }

            let num_vectors = period;
            let mut vec_list = Vec::with_capacity(num_vectors);
            for v in 0..num_vectors {
                let mut chunk = [0u8; 16];
                chunk.copy_from_slice(&pattern[v * 16..(v + 1) * 16]);
                vec_list.push(chunk);
            }
            masks[idx] = vec_list;
        }

        Self { masks }
    }

    #[inline(always)]
    pub unsafe fn sieve_tiny_primes(&self, sieve_buf: &mut [u8; SEGMENT_BYTES], seg_idx: u64) {
        let ptr = sieve_buf.as_mut_ptr();

        for (idx, &p) in TINY_PRIMES.iter().enumerate() {
            let vec_masks = &self.masks[idx];
            let period = p as usize;
            let mut phase = (seg_idx % (period as u64)) as usize;

            for offset in (0..SEGMENT_BYTES).step_by(16) {
                let mask_ptr = vec_masks.get_unchecked(phase).as_ptr();
                let v_data = vld1q_u8(ptr.add(offset));
                let v_mask = vld1q_u8(mask_ptr);
                let v_res = vandq_u8(v_data, v_mask);
                vst1q_u8(ptr.add(offset), v_res);

                phase += 1;
                if phase == period { phase = 0; }
            }
        }
    }
}

Step 4: Vector Popcount & Integration into d_worker.rs
Replace Mod-2 bit counting with NEON byte popcount across the 16 KiB buffer:
use std::arch::aarch64::*;
use crate::wheel30::SEGMENT_BYTES;

#[inline(always)]
pub unsafe fn wheel30_popcount_neon(sieve_buf: &[u8; SEGMENT_BYTES]) -> u64 {
    let ptr = sieve_buf.as_ptr();
    let mut acc = vdupq_n_u16(0);

    for i in (0..SEGMENT_BYTES).step_by(32) {
        let q0 = vld1q_u8(ptr.add(i));
        let q1 = vld1q_u8(ptr.add(i + 16));

        let cnt0 = vcntq_u8(q0);
        let cnt1 = vcntq_u8(q1);

        let sum0 = vpaddlq_u8(cnt0);
        let sum1 = vpaddlq_u8(cnt1);

        acc = vaddq_u16(acc, vaddq_u16(sum0, sum1));
    }

    vaddlvq_u16(acc) as u64
}

Verification and Benchmark Protocol
 * Register modules in crates/titan-sieve/src/lib.rs:
   pub mod wheel30;
pub mod wheel30_dense;
pub mod wheel30_tiny;

 * Build and verify test parity:
   cargo test --release -p titan-sieve

 * Run the isolated 10^{18} benchmark on cooled silicon:
   # Check thermal sensors and clock frequency
cat /sys/devices/system/cpu/cpu6/cpufreq/scaling_cur_freq
# Must read 2208000 (2.21 GHz)

cargo build --release --bin head_to_head_ultra
sleep 30
./target/release/head_to_head_ultra 1e18

Projected Delta from Step 2 (Wheel-30)
 * Physical Marks in D: Slashed from 2.09 \times 10^{11} down to 8.61 \times 10^{10} (2.43\times instruction reduction).
 * Segment Volume at 10^{18}: Slashed from 693,540 down to 369,888 segments (1.875\times fewer segment iterations).
 * Target Latency at 10^{18}: Expected to drop from 49.68s down to ~37.5s–40.0s, decisively overtaking primecount's 48.77s on physical silicon.

