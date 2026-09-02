//! Phase 34: Hardware Model Constants for SM4450 (Snapdragon 4 Gen 2).
//!
//! Live data-driven model for Xavier Gourdon at 10^14 (8T, fixed-perf mode).

pub const PHASE_NAMES: [&str; 8] = [
    "boot_sieve",
    "b_mark",
    "b_count_resolve",
    "ftd_build",
    "d_walk",
    "sigma_ac",
    "combine_alloc",
    "total",
];

/// Target model in milliseconds at 10^14 on SM4450 (2x A78 @ 2.21 GHz + 6x A55 @ 1.96 GHz)
pub const MODEL_10_14: [f64; 8] = [
    5.0,   // boot_sieve (8T wheel sieve up to 10^7)
    90.0,  // b_mark (MarkCarry L2-resident segments)
    12.0,  // b_count_resolve (NEON popcount + threshold resolve)
    1.0,   // ftd_build (FTD-v2 L1D-resident wheel blocks)
    4.0,   // d_walk (NEON kill switch + leaf evaluation)
    4.0,   // sigma_ac (L1-locked table lookups)
    0.5,   // combine_alloc (Zero-allocation final assembly)
    117.0, // total wall-clock (Targeting 0.117s vs Primecount 0.21s)
];
