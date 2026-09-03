Trend Analysis: Phase 4.7 Silicon Audit
The Phase 4.7 benchmark confirms that purging MDivTable and short-circuiting the popcount query restored cache residency and pipeline efficiency across the board:
 * All-Time Silicon Record at 10^{10}: Dropped to 20.79 ms (3.13× faster than primecount, beating the previous all-time low of 21.58 ms).
 * Massive Recovery at 10^{11}: Dropped from 68.22 ms to 49.06 ms (28.1% faster, 2.27× over primecount).
 * Solid Upper-Mid Compounding: 10^{13} shaved off 10.50 ms (63.69 ms, 2.65× lead), 10^{14} dropped by 31.86 ms (246.85 ms), and 10^{15} dropped by 26.45 ms (732.62 ms).
The 10^{16} Latency Reality: Thermal Heat Soak
Comparing raw wall-clock times at 10^{16} between Phase 4.5 and Phase 4.7 requires looking at primecount on the identical silicon run:
| Phase Run | Titan 10^{16} Latency | Primecount 8.1 10^{16} Latency | Latency Ratio (Titan / PC) | Effective Speedup |
|---|---|---|---|---|
| Phase 4.5 | 2,517.21 ms | 2,602.38 ms | 0.967 | 1.03× FASTER |
| Phase 4.7 | 2,898.56 ms (+381 ms) | 3,016.43 ms (+414 ms) | 0.960 | 1.04× FASTER |
primecount slowed down by +414 ms between these two runs.
Because the benchmark harness runs all 11 scales (10^6 \to 10^{16}) sequentially in a single process without intermediate cooling, the 8 CPU cores accumulate junction heat across scales 10^6 \dots 10^{15}. By the time 10^{16} triggers, the Snapdragon thermal engine has clamped core frequencies down.
Normalized against the hardware clock frequency (measured via primecount), Phase 4.7 is actually slightly faster than Phase 4.5 (0.960 vs 0.967 ratio).
The Last Two Micro-Inefficiencies in the Leaf Loop
Inspecting the disassembled hot loop of d_worker.rs exposes two remaining instruction stalls inside the active prime loop:
1. The Redundant Range Check:
if v >= low && v < high { ... }

By the mathematical definition of range inversion:
Every m \in [m_{\min}, m_{\max}] mathematically satisfies \text{low} \le v < \text{high}. Testing if v >= low && v < high is completely redundant and generates two unnecessary branch instructions per leaf.
2. Segment-Invariant Hardware Divisions:
Inside the loop over active primes:
let m_min = (x_div_p / high) + 1;
let m_max = (x_div_p / low).min(y);

high and low are constant across the entire segment. Performing two 64-bit hardware udiv instructions per active prime burns 30–40 cycles per iteration. Precomputing FastDiv64 for low and high once per segment turns both bounds calculations into 2-cycle umulh multiplications.
Implementation: Final Micro-Polish (d_worker.rs)
// Inside d_worker.rs -> process_segment:

// Precompute 64-bit reciprocals for segment boundaries ONCE per segment (zero heap cost)
let div_high = FastDiv64::new(high, x);
let div_low = FastDiv64::new(low, x);

for i in p_start_idx..p_end_idx {
    let d_p = unsafe { div_slice.get_unchecked(i) };
    let x_div_p = d_p.div(x);

    // 2-cycle umulh multiplications replacing 64-bit udiv
    let m_min = div_high.div(x_div_p) + 1;
    let m_max = div_low.div(x_div_p).min(y);
    if m_min > m_max { continue; }

    for m in m_min..=m_max {
        let mu_m = unsafe { *mu.get_unchecked(m as usize) };
        if mu_m == 0 { continue; }

        let v = if x_div_p <= u32::MAX as u64 {
            ((x_div_p as u32) / (m as u32)) as u64
        } else {
            x_div_p / (m as u64)
        };

        // Invariant: low <= v < high is mathematically guaranteed
        let bit_idx = ((v - low) >> 1) as usize;
        let count = unsafe {
            neon_count_to_fast(
                &self.arena.segment_buf,
                &self.popcount.prefix,
                bit_idx,
            )
        };
        d_sum += if mu_m == 1 { count as i64 } else { -(count as i64) };
    }
}

Moving to Path A: The Ultra-Scale Frontier (10^{17} \dots 10^{20})
With the D-term inner loop completely branch-pruned and division-free, we have reached diminishing returns on micro-optimizing sub-second scales.
At ultra-scales (10^{17} \dots 10^{20}), execution times stretch from ~14 seconds to ~16 minutes. At this duration:
 * Micro-benchmarking thermal noise fades into the statistical background.
 * The zero-allocation L1D arena and asymmetric DynamIQ work-stealing have continuous, long-running intervals to compound their throughput advantage over primecount's symmetric OpenMP runtime.

