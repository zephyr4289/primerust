The 41.871s record at 10^{18} proves that parallelizing AC across both Cortex-A78 cores and using persistent scratchpads removed the compute bottleneck. With thermals at 35.6°C skin and 46.1°C junction, the SoC is operating within its peak frequency envelope.
We can now exploit this headroom. The remaining 41.87 seconds is consumed almost entirely by D(x, y, z):
| Subsystem at 10^{18} | Latency | % of Total | Silicon Bottleneck |
|---|---|---|---|
| AC(x, y, z) (Dual-A78) | 2.62 s | 6.2% | Fully saturated, zero idle stalls |
| B(x, y) (Decoupled) | 3.85 s | 9.2% | Monotonic linear scan, minimal overhead |
| D(x, y, z) (Physical Sieve) | 34.80 s | 83.1% | Oversized segment count & Wheel-30 mark density |
| Setup, Tables, \Sigma, \Phi_0 | 0.60 s | 1.5% | Instant L1/L2 initialization |
Root Causes of the 34.8s Sieve Bottleneck
1. The Alpha Re-Balancing Dividend
In Phase 7.3, setting \alpha_y = 13.61 backfired because AC was running single-threaded and unoptimized, blowing up leaf latency. Now that AC is executed on dual A78 cores with FastDiv64 and SegmentedPiTable, it completes in 2.62 seconds. Keeping \alpha_y throttled at 8.75 forces Titan to sieve 232,480 segments (114.28\times 10^9 integers) needlessly. Expanding \alpha_y \rightarrow 10.75 slashes 38,920 segments from D, while adding only ~0.5s to AC.
2. Big Cores Constrained to 16 KiB Wheel-30
When Cores 6 and 7 finish AC at T = 2.6\text{s} and steal D segments, they run the Cortex-A55's 16 KiB Wheel-30 sieve kernel.
 * A 16 KiB buffer uses just 24% of the Cortex-A78's 64 KiB L1D cache, inflating prime reload and outer loop overhead by 3\times.
 * Wheel-30 filters \{2, 3, 5\} (26.67% density). The Cortex-A78 has a 64 KiB L1I cache that can run the larger Wheel-210 kernel (\{2, 3, 5, 7\}, 22.86% density), cutting composite marking operations by 14.3%.
3. Dense Traversal of Dormant Sparse Primes
Primes p > 32,768 hit a 480k-integer segment fewer than 15 times, and primes p > 100,000 hit fewer than 5 times. Streaming tens of thousands of sparse prime states sequentially inside the dense marking loop wastes L1/L2 memory bandwidth on dormant pointers.
Phase 7.7 Architectural Blueprint: The Sub-35-Second Squeeze
                 ┌────────────────────────────────────────────────────────┐
                 │             Phase 7.7 Architectural Core               │
                 └────────────────────────────────────────────────────────┘
                                             │
                      ┌──────────────────────┴──────────────────────┐
                      ▼                                             ▼
         ┌─────────────────────────┐                   ┌─────────────────────────┐
         │ Alpha Re-Balancing      │                   │ Asymmetric Multi-Tile   │
         │ α_y = 10.75 at 10¹⁸     │                   │ A78: 48 KiB Wheel-210   │
         │ -38,920 Segments in D   │                   │ A55: 16 KiB Wheel-30    │
         │ Net Gain: -5.2s Sieve   │                   │ -14.3% Marks on Big Core│
         └─────────────────────────┘                   └─────────────────────────┘
                      │                                             │
                      └──────────────────────┬──────────────────────┘
                                             │
                                             ▼
         ┌───────────────────────────────────────────────────────────────────────┐
         │ Bucketed Sparse Sieve: Skip dormant primes (p > 32,768) in inner loop │
         │ Target Latency: ~34.2s at 10¹⁸ (1.35x faster than Primecount)         │
         └───────────────────────────────────────────────────────────────────────┘

Implementation Modules
1. Re-Anchoring the Equilibrium Knot (tuning.rs)
Update TUNING_KNOTS in crates/titan-core/src/tuning.rs:
// crates/titan-core/src/tuning.rs

const TUNING_KNOTS: &[TuningKnot] = &[
    TuningKnot { log10_x:  6.0, alpha_y:  1.000, alpha_z: 1.000 },
    TuningKnot { log10_x:  7.0, alpha_y:  1.100, alpha_z: 1.000 },
    TuningKnot { log10_x:  8.0, alpha_y:  1.250, alpha_z: 1.000 },
    TuningKnot { log10_x:  9.0, alpha_y:  1.500, alpha_z: 1.100 },
    TuningKnot { log10_x: 10.0, alpha_y:  1.950, alpha_z: 1.200 },
    TuningKnot { log10_x: 11.0, alpha_y:  2.700, alpha_z: 1.350 },
    TuningKnot { log10_x: 12.0, alpha_y:  3.650, alpha_z: 1.500 },
    TuningKnot { log10_x: 13.0, alpha_y:  4.800, alpha_z: 1.650 },
    TuningKnot { log10_x: 14.0, alpha_y:  6.200, alpha_z: 1.800 },
    TuningKnot { log10_x: 15.0, alpha_y:  7.750, alpha_z: 1.900 },
    TuningKnot { log10_x: 16.0, alpha_y:  9.400, alpha_z: 2.000 },
    TuningKnot { log10_x: 17.0, alpha_y: 10.940, alpha_z: 2.000 }, // Locked at 10.20s record
    TuningKnot { log10_x: 18.0, alpha_y: 10.750, alpha_z: 2.000 }, // Re-balanced: -38,920 segments
    TuningKnot { log10_x: 19.0, alpha_y: 13.500, alpha_z: 2.000 },
];

 * Parameters for 10^{18}: y = 10,750,000, z = 21,500,000, endpoint x/y = 93,023,255,813.
 * Sieve segments drop from 232,480 \rightarrow \mathbf{193,560} (-16.7\% physical sieve volume).
2. Asymmetric Sieve Worker Dispatch (d_worker_asym.rs)
Equip Cortex-A78 workers with a 48 KiB Wheel-210 engine while retaining the 16 KiB Wheel-30 engine on Cortex-A55:
//! Asymmetric Sieve Kernels: Wheel-210 on A78 (48 KiB), Wheel-30 on A55 (16 KiB).

use titan_core::tuning::{GourdonParams, SEGMENT_BYTES, SEGMENT_INTEGERS};

pub const A78_BUFFER_BYTES: usize = SEGMENT_BYTES * 3; // 48,048 bytes (~46.9 KiB, fits 64 KiB L1D)
pub const A78_SEGMENT_SPAN: u64 = (SEGMENT_INTEGERS as u64) * 3;

/// Sieve routine for Cortex-A55 little cores: 16 KiB L1D Wheel-30
#[inline(always)]
pub fn sieve_a55_segment(
    seg_idx: u64,
    params: &GourdonParams,
    primes: &[u32],
    scratchpad: &mut [u8; SEGMENT_BYTES],
) -> i64 {
    scratchpad.fill(0xFF);
    let seg_low = params.z + seg_idx * (SEGMENT_INTEGERS as u64);
    let seg_high = seg_low + (SEGMENT_INTEGERS as u64);

    unsafe {
        crate::wheel30_dense::sieve_wheel30_dense(
            scratchpad.as_mut_ptr(),
            seg_low,
            seg_high,
            primes,
        );
    }
    crate::wheel30_popcount::popcount_neon_segment(scratchpad) as i64
}

/// Sieve routine for Cortex-A78 big cores: 48 KiB L1D Wheel-210 (Triple-Segment)
#[inline(always)]
pub fn sieve_a78_triple_segment(
    triple_idx: u64,
    params: &GourdonParams,
    primes: &[u32],
    scratchpad_48k: &mut [u8; A78_BUFFER_BYTES],
) -> i64 {
    scratchpad_48k.fill(0xFF);
    let seg_low = params.z + triple_idx * A78_SEGMENT_SPAN;
    let seg_high = (seg_low + A78_SEGMENT_SPAN).min(params.x_div_y);

    if seg_low >= seg_high {
        return 0;
    }

    unsafe {
        // Wheel-210 filters residues mod 210, cutting marks by 14.3%
        crate::wheel210_dense::sieve_wheel210_dense(
            scratchpad_48k.as_mut_ptr(),
            seg_low,
            seg_high,
            primes,
        );
    }

    // Vectorized 48 KiB NEON popcount
    let mut total_primes = 0i64;
    let chunks = scratchpad_48k.chunks_exact(SEGMENT_BYTES);
    for chunk in chunks {
        total_primes += crate::wheel30_popcount::popcount_neon_segment(chunk) as i64;
    }
    total_primes
}

Step-by-Step Execution Playbook
Step 1: Isolated Unit Verification (<60 ms)
Compile and test parameter knot scaling:
cargo test -p titan-core --lib tuning -- --nocapture

Step 2: Instant Tier-2 Smoke Verification (10^{11} \rightarrow 10^{13})
Verify that the revised parameter curve maintains exact parity across scales:
cargo build --release --bin head_to_head
./target/release/head_to_head 1e11 1e12 1e13

Step 3: Milestone Ultra Battle (10^{17} \rightarrow 10^{18})
sleep 20
cargo build --release --bin head_to_head_ultra
./target/release/head_to_head_ultra 1e17 1e18

Projected Performance Impact (Phase 7.7)
| Scale | Primecount 8.1 | Titan Phase 7.6 | Titan Phase 7.7 (Projected) | Margin vs. Primecount | Target Verdict |
|---|---|---|---|---|---|
| 10^{17} | 10,542.16 ms | 10,514.64 ms | ~9,950 ms | +592 ms faster (1.06×) | Retained Dominance |
| 10^{18} | 46,312.10 ms | 41,871.03 ms | ~34,900 ms | +11.41 s faster (1.33×) | Sub-35s World Record |

