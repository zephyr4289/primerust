Root Cause: The Phase 7.0 Workload Inversion
Phase 7.0 pushed \alpha_y aggressively to 13.61 at 10^{18} (y = 13,609,000). While this eliminated 89,881 segments in D, it triggered an arithmetic explosion in A and C.
 * Non-Linear Leaf Explosion in C(x, y):
   The number of leaf evaluations and integer divisions in Gourdon's C and A terms scales roughly as \mathcal{O}(y \cdot \pi(\sqrt{x/y})) and \mathcal{O}(y \log y). Increasing y by 2.68\times (from \alpha_y \approx 5.0 to 13.61) expanded the AC iteration space by over 4.5\times.
 * The Cortex-A55 UDIV Trap:
   On the Snapdragon 4 Gen 2 (SM4450), 6 out of 8 cores are Cortex-A55. The A55 is an in-order core where 64-bit integer division (udiv) is non-pipelined, stalling execution for 12–19 cycles. At hundreds of millions of divisions, the CPU burned tens of billions of cycles waiting on division execution units. The sieve in D (streamed bitwise ops) was cheap; the divisions in AC were crippling.
 * Cache Misses in \pi(v) Lookups:
   Evaluating leaves in C requires determining \pi(x / (p \cdot q)). When y expanded past 13 million, lookup ranges blew past the 32 KiB L1D cache of the A55, stalling the pipeline on L2/System-Level Cache fetches.
Phase 7.1 Architectural Blueprint
To convert the sieve segment reduction into a net performance win, Phase 7.1 focuses on zero-hardware-division execution and L1D-resident counting tables.
                ┌────────────────────────────────────────────────────────┐
                │             Phase 7.1 Architecture Pipeline            │
                └────────────────────────────────────────────────────────┘
                                            │
                     ┌──────────────────────┴──────────────────────┐
                     ▼                                             ▼
        ┌─────────────────────────┐                   ┌─────────────────────────┐
        │   FastReciprocal<u64>   │                   │  L1D-Locked PiTable     │
        │  Replaces 64-bit UDIV   │                   │  Compressed 2-Level     │
        │  with ARM64 UMULH + SRL │                   │  Stride = 64 / 2048     │
        └─────────────────────────┘                   └─────────────────────────┘
                     │                                             │
                     └──────────────────────┬──────────────────────┘
                                            │
                                            ▼
        ┌───────────────────────────────────────────────────────────────────────┐
        │ DynamIQ Asymmetric Scheduler: Heavy AC -> A78 | Streaming D -> A55   │
        └───────────────────────────────────────────────────────────────────────┘

1. Invariant Division Replacement via Granlund-Montgomery (UMULH)
Every division by a prime p in the inner loops of A and C must be eliminated. On ARM64 (AArch64), an unsigned 128-bit widening multiply produces the high 64 bits in a single instruction (UMULH), taking 3–4 cycles on Cortex-A78 and 2–3 cycles on Cortex-A55, fully pipelined.
// libdivide-style invariant fast division for inner AC loops
#[derive(Copy, Clone)]
pub struct FastDiv64 {
    multiplier: u64,
    shift: u8,
}

impl FastDiv64 {
    #[inline(always)]
    pub const fn new(d: u64) -> Self {
        assert!(d > 1);
        // Compute ceil(2^(64 + shift) / d)
        let l = 64 - (d - 1).leading_zeros();
        let shift = l as u8;
        let m = (((1u128 << (64 + shift)) + (d as u128 - 1)) / (d as u128)) as u64;
        Self { multiplier: m, shift }
    }

    #[inline(always)]
    pub fn divide(&self, n: u64) -> u64 {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            use core::arch::aarch64::*;
            // Maps directly to: umulh x0, x1, x2; lsr x0, x0, shift
            let hi: u64;
            core::arch::asm!(
                "umulh {hi}, {n}, {m}",
                hi = out(reg) hi,
                n = in(reg) n,
                m = in(reg) self.multiplier,
                options(pure, nomem, nostack)
            );
            hi >> self.shift
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            (((n as u128) * (self.multiplier as u128)) >> 64) as u64 >> self.shift
        }
    }
}

Pre-computation: In C(x, y), primes p \le y are iterated sequentially. Maintain a pre-allocated vector reciprocals: Vec<FastDiv64> generated alongside the prime list up to y. In the inner loop, division becomes one umulh and one register shift.
2. Cache-Locked Two-Level SegmentedPiTable
Evaluating \pi(v) where v = \lfloor x / (p \cdot q) \rfloor must avoid random cache accesses. For v up to z = 2y, use a two-tier relative indexing layout fitting strictly within the Cortex-A55 32 KiB L1D.
 * Coarse Counter (Tier 1): Absolute count stored every 2,048 integers (1 u32 per 2,048 span).
 * Fine Counter (Tier 2): Bitmask of primes using Wheel-30 layout.
 * Lookup Cost: Exactly 1 load from Tier 1 + 1 bitmask load from Tier 2 + 1 POPCNT (cnt / addv NEON instruction).
[Tier 1: Base Count (u32)] ──> Base π at block boundary
                                   │
                                   ▼
[Tier 2: Packed Bitmask]   ──> POPCNT(mask & residue_mask)
                                   │
                                   ▼
                               Total π(v) (0 pipeline stalls)

3. DynamIQ Heterogeneous Worker Affinity
The Snapdragon 4 Gen 2 has two Cortex-A78 cores (2.2 GHz) and six Cortex-A55 cores (2.0 GHz). Running homogeneous work across all 8 cores creates a high-latency tail because A55 cores lag on AC tree evaluations.
 * A78 Worker Group (Cores 6–7): Assigned exclusively to A(x, y) and the highest branch-density portions of C(x, y). The A78's out-of-order execution engine absorbs mispredicted branches and un-vectorized leaf branches easily.
 * A55 Worker Group (Cores 0–5): Assigned to the segmented sieve of D. Sieve operations consist of deterministic bit-setting (BFI, ORR, STR) with linear access patterns that prefetch reliably into L1D without stalling in-order pipelines.
4. Alpha Knot Re-Convergence
Phase 7.0 shifted too much weight to AC because the knot curve was calibrated assuming D was the dominant cost across all scales. Now that D is cut, adjust \alpha_y down to balance the cross-over point until AC is accelerated.
       Phase 6.13:   α_y = 5.2  (D is bottleneck, AC is tiny)
       Phase 7.0 :   α_y = 13.6 (D is cut by 40%, AC explodes 400% -> Net Loss)
       Phase 7.1 :   α_y = 8.5  (Equilibrium with FastDiv64 + L1D PiTable)

| Parameter | Phase 6.13 | Phase 7.0 | Phase 7.1 Target |
|---|---|---|---|
| \alpha_y at 10^{17} | 4.85 | 10.94 | 7.20 |
| \alpha_y at 10^{18} | 5.60 | 13.61 | 8.50 |
| y at 10^{18} | 5,600,000 | 13,609,000 | 8,500,000 |
| Sieve Segments (D) | 239,323 | 149,442 | 182,100 |
| Division Strategy | Hardware UDIV | Hardware UDIV | Branchless UMULH |
| PiTable Lookup | Flat array | Flat array | 2-Level L1D Packed |
Implementation Tasks for 7.1
 * src/arithmetic/fast_div.rs:
   Implement FastDiv64 with an AArch64 inline assembly path using umulh. Pre-generate divisors for all primes in the base sieve buffer p \in [2, y].
 * src/gourdon/c_term.rs & a_term.rs:
   Replace all instances of / p and / q with fast_div.divide(...). Ensure registers holding reciprocal constants remain pinned inside the loop registers (x19-x28).
 * src/tables/pi_table_l1.rs:
   Implement the 2-level 32 KiB cache-capped counting table with hardware popcount intrinsics (core::arch::aarch64::vcnt_u8).
 * src/tuning/knot.rs:
   Re-anchor the Monotone Cubic Hermite Knot at 10^{17} (\alpha_y = 7.20) and 10^{18} (\alpha_y = 8.50) to find the true compute-sieve equilibrium.

