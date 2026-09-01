# Titan-Prime: The Engineering Order of Battle

This ledger is exactly the right move — you now have **measured ground truth on your own silicon**. Before the list, three facts hiding inside your own numbers that must dictate every design decision:

## 1. What Your Own Data Is Telling You

| Observation | Engineering Implication |
|---|---|
| 8 threads → only **3.08×** scaling (6.858B vs 2.225B/s) | That's big.LITTLE math: 2×A76 @ ~2.2B each + 6×A55 @ ~0.4B each ≈ 6.85B. **A55 ≈ 18% of an A76.** Equal work division caps you at half speed — weighted partitioning (~5.5:1) is mandatory, not optional |
| 6.858B/s burst → **4.235B/s sustained** at 10¹¹ (−38%) | Thermal droop is a first-class design citizen. Every benchmark must report *sustained*, and the scheduler must be throttle-aware |
| primecount: π(10¹⁶) in 2.756s | **That's the summit on this device.** Everything in titan-count is a climb toward that number |

Also note: A76 has 64KB L1D, A55 has 32KB L1D. **Heterogeneous cores deserve heterogeneous segment geometry** — something primesieve does not do. That's a real differentiator lane.

## 2. Spec Bugs to Freeze Before Coding (file_structure.md)

1. **Wheel bit layout is internally inconsistent.** You correctly list the coprime residues {1,7,11,13,17,19,23,29}, but your bit table maps bit0→+7 ... bit7→+31 (next block's residue 1), silently dropping residue 1. Both phase choices are *valid wheels*, but they're *different wheels* — copy a presieve pattern assuming one while implementing the other and you get silent bit corruption at every segment boundary. Pick ONE, document it, make `wheel.rs` the single source of truth, exhaustive-test the first 1,000 primes bit-exact.
2. **Fenwick tree claim — verify.** I believe primecount uses direct precomputed prefix tables (O(1) lookup); Fenwick's O(log n) only earns its keep with mutable point-updates, which the π-table doesn't have. Don't pay a log-factor where a prefix array gives O(1).
3. **The Gourdon decomposition (AC − B + D + Φ₀ + Σ)** must be verified term-by-term against primecount's actual source + Gourdon's paper *before* `gourdon.rs` exists. One wrong sign = wrong answers discovered weeks later.
4. **PhiTiny k≤8 is a cache-budget lie.** P₆ = 30,030 (60KB as u16 — fine). P₇ = 510,510 (~1MB). P₈ = 9.7M (~19MB — not "tiny"). Size the tables to cache budget, not ambition.
5. **`core::simd` is nightly-only.** On stable Rust: cfg'd `std::arch::aarch64` (NEON is baseline on aarch64, zero detection needed) + `is_x86_feature_detected!` for x86 backends. Decide now, or you're hostage to nightly churn.

## 3. THE LIST — One by One

**Phase 0 — Instrument Before Engineering** *(titan-oracle + bench harness)*
- Frequency **canary**: fixed-cost work unit run before/after every benchmark → normalize for throttling (no root needed, unlike cpufreq sysfs)
- Per-core survey: pin identical work to each of the 8 cores → throughput vector that becomes the load balancer's constants
- Oracle v0: A006880 table + trial-division reference (slow but *obviously* correct) + subprocess diff vs your primesieve/primecount binaries
- Benchmark hygiene protocol: `termux-wake-lock`, battery >50%, never while charging, same screen state every run, min-of-N + sustained-of-N both reported
- Fill ledger gaps (primecount @ 10¹⁰, 10¹¹) so every future comparison has a measured reference
- **GATE**: oracle catches an intentionally-injected off-by-one (mutation test green — proving the *harness* works)

**Phase 1 — titan-core** *(correctness primitives)*
- `roots.rs`: integer isqrt/icbrt/iroot4 with correction loops, property-tested vs u128 — never `pow(x, 1.0/4)` again
- `phi_tiny.rs`: periodic-identity tables, brute-force diff for all x ≤ 10⁶
- `bit_array.rs` + **zero-alloc discipline**: global counting allocator asserts 0 allocations in steady-state
- **GATE**: mutation tests green + zero-alloc assertion green

**Phase 2 — titan-sieve, Single-Threaded: Correct → Fast** *(presieve, erat_small, erat_medium, segment)*
- v0 scalar: segmented wheel-30, 32KiB, presieve pattern, popcnt tally — **GATE: bit-exact π(10⁹)**
- Then the optimization ladder, ONE change per commit, keep-or-revert at ±3%: NEON presieve → 8 unrolled EratSmall residue loops → NEON medium marking → vector `CNT` popcount → segment-size sweep (32/64/128KiB)
- **GATE: ≥1.5B/s single-thread** (70% of primesieve's 2.225B/s = a strong v1 — it's decades-tuned)

**Phase 3 — titan-pool: Heterogeneous Multi-Threading**
- Weighted static partition from Phase 0's measured vector + work-stealing fallback; `sched_setaffinity` pinning via libc
- **The thermal experiment**: sweep thread mixes (2 big / +1…+6 little), measure 60s *sustained* throughput, find the mix that beats primesieve's 4.235B/s sustained. primesieve is not throttle-aware; we can be. **This is the winnable lane**
- Dual segment geometry: 64KiB on A76, 32KiB on A55 (legal since we only tally counts per thread)
- **GATE: π(10¹¹) exact + sustained aggregate ≥ 4.235B/s on-device**

**Phase 4 — erat_big: Buckets Unlock Deep-N** *(bucket, erat_big)*
- Mandatory past ~10¹³: at 10¹⁴, √N = 10⁸ and nearly all sieving primes skip any single 32KiB segment — bucketed queues are the only way to avoid 99.9% no-op iteration
- Zero-alloc bucket arena from a fixed memory pool
- **GATE: exact streaming/count at 10¹³ and 10¹⁴**

**Phase 5 — titan-count: The Ladder, Not the Elevator**
- v0 **Lehmer** with dense π/φ tables (no hash caches, no doubles) — **GATE: π(10¹²) exact < 1s**, finally killing the ghost of seive.md
- v1: hierarchical cache-hard φ tables + segmented P2 — **GATE: π(10¹⁴) < ~5s**
- v2: Gourdon terms (after source-verification), term-level DIY tests since primecount only exposes the final answer
- Bonus: `primecount --algorithm=` exposes Lehmer/LMO/Gourdon variants — benchmark each rung on-device so every ladder step has its own reference time
- **GATE: π(10¹⁶) = 279,238,341,033,925 exactly.** Then the speed climb toward 2.756s — within 2–3× is an excellent result for a from-scratch build

**Phase 6 — Cross-Arch & Product** *(titan-cli, CI)*
- cfg'd SIMD backends + runtime dispatch; CI via cross/QEMU + aarch64 runners; CLI count/stream/bench modes with JSON output the oracle can parse

## 4. Rules of Engagement

- **Oracle green = merge criterion.** One phase per branch. No exceptions — the moment you merge red "to fix later," you no longer have an engine, you have a belief system
- **The ledger is law**: every optimization logged with canary-normalized numbers, or it didn't happen
- No `unsafe` outside tightly-wrapped SIMD kernels
- Sweep primesieve's own knobs (`--sieve-size`, `-t`) to find its *best* sustained config on-device — you must beat its best, not its default

The honest summit ladder: **correct** → **credible** (70% of 1T primesieve) → **parity** (sustained MT) → **breakthrough** (thermal-aware sustained beats it). The first two are engineering discipline; the last one is the phone-specific edge nobody else has built.

Say go and I'll drop the Phase 0 harness + `wheel.rs` skeleton with the bit-layout tests baked in.
