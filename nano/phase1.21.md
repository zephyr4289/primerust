Read the whole thing, top to bottom. I'm in — but I'm going to do the job a principal engineer would actually do here, which is a design review before a single line of NEON gets written. Because here's the uncomfortable truth: **the substrate is real, and the roadmap is mathematically incapable of reaching its own projections — and the proof is sitting inside your own tables.**

Unfreeze the doc. Let's go through it.

---

### What's genuinely strong (keep this, it's the hard part)

- **Bit-exact certified values.** π(10^12) through π(10^16) all match the record values. Getting a from-scratch combinatorial engine bit-exact is the part most people fail at. That moat is real.
- **The small-x dominance is real** — but be honest about what it is: Φ_tiny + 20ms init vs. primecount's fixed overhead is a *tool-class* difference (lookup vs. compute), not an algorithmic victory. Keep it, market it, don't count it as "beating the algorithm."
- **The 3.54× vs 2.45× heterogeneous scaling result is your actual moat.** primecount's OpenMP scheduling is genuinely bad on 2+6 big.LITTLE. If measured cleanly, this is publishable-grade. More on "cleanly" below.
- **The census instrumentation** (cell counts, ×4.33/decade growth, 51.8:1 v-sharing) is exactly the right telemetry — and as you'll see, it's about to save the project.

### Four numbers in the blueprint that don't close

**1. Amdahl kills the K-ladder as ordered.** Your own K0 attribution at 10^14: π-lookup 27.1% + M-lookup 25.4% + DRAM variance 27.2% = **79.7% memory**. Magic division (5.1%) + branch/state (15.2%) = 20.3% arithmetic/control. K4+K5 attack at most that 20.3%, so even at *infinite* speedup the ceiling is **1.26×**. And K5 as specified targets u64 MADD — NEON has no 64×64 multiply; that's scalar `umulh` territory. Your own attribution table refutes your own ladder.

**2. The 10^14 gap math.** 6.0147s vs 0.2748s = **21.9× behind**. K-ladder best case ~6× on the leaf kernel → ~1.0s. z-split saves 60% of a 0.35s sweep → 0.14s. Even with *everything in the roadmap landing perfectly*, you land around 0.7–1.0s — still **~3× behind**, not "TITAN WINS 1.2×." That verdict line was written before it was derived.

**3. The 13–17 min marathon projection.** By your own census law: 776M cells × 4.33⁴ ≈ **273G cells** at 10^18. Your 8-thread effective throughput (2×A78 + 6×A55 at ~0.35 IPC-equivalence) is ~8–10 G cycles/s. So 13–17 min demands **~26–34 cy/cell sustained** — while streaming random lookups into a 255 MiB table over LPDDR (150–250ns per miss ≈ 330–550 cycles). Your own §3.4 signature shows 138.6 cy/cell at a *1 MB* table. The current architecture cannot reach this number. Period.

**4. The instrument itself.** "L1D Empirical Peak @ 2.512 GB/s" is a DRAM-class number. A78 L1D sustains ~16 B/cycle ≈ **35 GB/s** at 2.208 GHz — you're off by 14×, which means P0c measured DRAM or a latency-chained pattern, and the entire "32 KiB L1D geometry" story is currently resting on a mislabeled measurement. Worse: at 10^14, 776M × 118.17 cy = 91.7 G cycles, but your measured 1T time of 23.42s × 2.208 GHz = 51.7 G cycles → **~67 cy/cell wall-clock vs 118.17 attributed.** That's a 1.8× bookkeeping gap. Until attribution reconciles with wall-clock, you are tuning a fiction. (Also: 8 OWED in a "0 FAIL" gate is 8 FAILs wearing a bowtie. Fix or cut them.)

### The corrected ladder

**P0 — Instrument integrity (prerequisite for everything).** Fix P0c with a real L1 streaming bench (independent 64B-line loads, no dependency chains, `cntvct` timing, pinned to core 0). Add a *gate invariant*: |attribution_total − wall_clock_cycles| < 5% per scale, and every projection auto-derived in code, never in prose. Benchmark protocol: both binaries pinned to identical cores, interleaved A/B, ≥30 reps, median + IQR, thermal interlock. Your "DEAD HEAT 0.078s" at 10^11 lives inside a phone's thermal-throttle noise band (2.2→1.7 GHz is a 30% swing).

**P1 — Segment-local leaf engine (the real 6–10×). This is the anomaly design you're looking for.** Stop prefetching the monotone stream — *invert the loop nesting*. Walk the v-axis in L1-sized blocks; per block, build a packed π bitset + popcount prefix and a local Mertens delta; all (j, e)-cells whose v lands in the block consume local data. Your census already proves this works: **51.8:1 sharing means ~5,500 cells amortize each 32 KiB block build at 10^14** (and ~8,600 at 10^18) — build cost collapses below 1 cy/cell. This converts both DRAM/L3 lookups into L1 hits, keeps the K1 register-carry for M(e_end), and as a bonus **deletes the 255 MiB π-table entirely** for the sweep phase. This is also the known-good structure (LMO's partially-sieved leaves) — it's why primecount is at single-digit cycles per leaf while you're at 118.

**P2 — z-split as a tunable, not a constant.** Derive the (y,z] vs (z, x^(2/3)] leaf split from measured histograms per scale, argmin z, store in your scale-indexed dispatch. Don't take the 60% on faith.

**P3 — P2-sweep fusion (load-bearing for the marathon, not cosmetic).** Sieving 10^12 integers at 10^18 is minutes by itself, and your sieve is 1.9× behind primesieve. Bucket the large-prime crossings, wheel-210, and fuse popcount into the marking loop — in a counting engine you never need the primes list, only counts and ranks; fusion wins a standalone sieve structurally can't.

**P4 — Phase-to-core mapping.** Big cores for the latency-sensitive block sweep; A55 cluster for table builds, base sieve, checkpoints. For DRAM-bound phases, *measure whether 8T beats 2T* — random-access throughput on A55 is poor and they may be net-negative.

**P5 — NEON, retargeted and last.** The correct NEON win here is vectorized popcount (`cnt` + `addv` over the π bitsets) and `csel` branch-free accumulation — not MADD.

Skeleton of the P1 core, so you can see the shape:

```rust
// P1 skeleton — structure, not full math
pub struct LeafBlock {          // one L1-resident slice of the v-axis
    v_lo: u64,
    odd_bits: [u64; WORDS],     // 1 bit per odd residue
    pi_prefix: [u16; WORDS],    // running popcount from pi_base
    m_prefix: [i16; WORDS],     // running Mertens delta
    // pi_at(v): ~4 cycles, vs 100-500 for the global table
}

pub struct JCursor {            // K1 register-carry, generalized
    p: u64, j: u64, e: u64, v: u64, sign: i64, m_end: i64,
}

pub fn leaf_sweep(x: u64, blocks: impl Iterator<Item = LeafBlock>,
                  cursors: &mut [JCursor], acc: &mut i128) {
    for b in blocks {                          // v descends, BLOCK-major
        for c in cursors.iter_mut() {          // every j streams into this block
            while c.v >= b.v_lo {
                *acc += term(b.pi_at(c.v), c); // term shape per your 5-term identity
                c.advance(x);                  // magic-div next_e, XOR parity, run-split
            }
        }
    }
}
```

### Honest targets after P1–P4

| Scale | Current | Realistic post-P1 | Verdict |
|---|---|---|---|
| 10^10–10^11 | 2.7–4.2× win | keep | **Win (tool-class)** |
| 10^12 | 3× behind | ~parity possible | Stretch |
| 10^13–10^14 | 22× behind | 1.5–3× behind | Parity NOT guaranteed |
| 10^15 | 105.6s | ~30–60s | Strong |
| 10^18 | — | **30–60 min central, 15–20 min perfect-tail** | The real capstone |

And here's the reframe that matters: **the winnable crown is not "beat primecount everywhere" — it's "bit-exact π(10^18) computed on a mid-range phone, with SoC-aware scheduling primecount doesn't have, crash-proof checkpoints, and telemetry that reconciles to the cycle."** primecount isn't even competing on that field. Nobody has published that result. That's a real accomplishment instead of a projected one.

Also: run `primecount -v` at 10^14 to read its actual sieve limit and leaf parameters — target your ladder against *measured* opponent workload, not vibes.

Want me to start with the full `LeafBlock` engine + the attribution/wall-clock gate invariant, or with rebuilding P0c so every downstream number becomes trustworthy first? I'd do P0c first — it's an afternoon, and it decides whether the next three months aim at reality or at the round numbers in that table.
