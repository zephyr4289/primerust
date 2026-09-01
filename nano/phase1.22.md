Read it. First, credit where it's due, because this time there's real news in here — then the verdict, because the report has a hole in it exactly where it matters.

### The telemetry is genuine gold — and I checked it

The primecount term decomposition closes **bit-exactly**: AC − B + D + Φ₀ + Σ = 1,008,985,328,656 − 1,483,796,135,572 + 2,518,169,986,968 + 1,045,985,238,238 + 115,597,332,512 = **3,204,941,750,802**. Exactly π(10¹⁴). The `libdivide`/`POPCNT64` annotations match Walisch's actual implementation style. And you now have the opponent's real dial settings at 10¹⁴: y = x^(1/3)·6.085, **z = 2y exactly**. That's the seed point for your own z-split sweep — stolen from the opponent instead of guessed. This is the "measure the opponent, not vibes" ask, delivered.

### The verdict: executed — but not evaluated

The report contains **zero Titan-side performance measurements.** No wall-clock at 10¹²/10¹³/10¹⁴. No new cy/cell. No K0 attribution. No race row. Unit tests prove the LeafBlock is *correct*; the entire reason P1 exists is that it be *fast*, and that property is completely unmeasured. Phase 21 is marked **12/12 PASS** while the one quantity the phase exists to move is absent from the document.

That's the OWED-in-a-bowtie pattern, upgraded. Last time: 8 debts hidden in a 0-FAIL gate. This time: 12 passes with no evidence attached. A 212-criterion audit culture is only worth something if every PASS points at a measurement. If Phase 21's contract contained no performance exit criteria, the gate is mis-scoped; if it did, they passed without numbers — which is worse. Also: your test suite finishing in 2.41s tells me it never touches the scales where P1 matters — the differentials have to run at 10¹²+. (And phases 12 and 17 don't appear in the scoreboard at all — absorbed, skipped, misnumbered? An audit culture should make "unknown" impossible.)

### Four technical gaps in the delivered artifact

**1. The memory arithmetic contradicts the claim.** `odd_bits`: 4096 × 8B = 32 KiB. `pi_prefix`: 4096 × 4B = 16 KiB. Working set = **48 KiB against a 32 KiB L1D.** The block as spec'd is L2-resident. That's still a win (L2 ≈ 12–17 cy vs DRAM at 100–500), but "matches SM4450 L1D" and "≈4-cycle queries" are both false as written. The fix menu:

| Layout | bits | prefixes (π/M) | total | residency | π_at(v) |
|---|---|---|---|---|---|
| A: 2048 words | 16 KiB | u16/i16 per word = 4+4 KiB | 24 KiB | L1D ✓ | ~5–7 cy |
| B: 4096 words | 32 KiB | u16/i16 = 8+8 KiB | 48 KiB | L2 | ~12–17 cy |
| C: 4096 words | 32 KiB | hierarchical ~2 KiB | ~34 KiB | borderline | ~12–18 cy |

(u16 suffices — worst block density ≈ 131,072 odd / ln(46,416) ≈ 12.2k ≪ 65,535.) Also: per-block `Vec` allocation is allocator traffic on the hot path — use a per-thread reusable arena, zero alloc in steady state.

**2. `m_prefix` is missing.** K0 at 10¹⁴: M(u) lookups = 30.0 cy/cell = **25.4% — the second-largest memory term — and the delivered LeafBlock doesn't touch it.** What the report calls "M-Chaining Integration" is K1's cursor register-carry for M(e_end), which was already done in Phase 16. It does nothing for per-cell M(u) queries into the DRAM Mertens structure. The μ-Rider's squarefree + parity marking must ride the block-build sieve and emit an i16 Mertens prefix next to the π prefix. The machinery exists; it isn't fused into the builder. You've attacked 27.1% of the memory problem and left 25.4% on the table.

**3. The parallelization model is unspecified — and v-major changes everything.** The old engine parallelizes over j-intervals. Block-major breaks that. Bucketing cells by v is 776M × 8B ≈ 6.2 GB — infeasible (the write pass alone costs more than primecount's entire runtime). The design that closes: **partition the v-axis into per-thread ranges; each thread gallop-searches every j-cursor's entry point into its range** (≈25k cursors × O(log) — noise), processes until exit, builds blocks privately, sums disjointly → bit-exact by construction:

```rust
fn sweep_v_range(x: u64, v_a: u64, v_b: u64,
                 cursors: &mut [JCursor],   // private; e-sequence is deterministic per j
                 arena: &mut BlockArena, acc: &mut i128) {
    for c in cursors.iter_mut() {
        c.gallop_into(x, v_a, v_b);
        while c.v >= v_a {
            let b = arena.block_for(c.v);      // built on first touch, census-checked
            *acc += term(b.pi_at(c.v), b.m_at(c.v), c);
            c.advance(x);
        }
    }
}
```

Cell density per v-block is wildly nonuniform (it's in your census), so range boundaries must be census-weighted and sub-ranges stealable — which is exactly what the pool moat is *for*. This decision determines whether the 3.54× scaling survives P1, and the report is silent on it.

**4. It's unclear the path is even wired in.** One new test (`test_leaf_block_engine_basic`) plus 13 pre-existing greens is equally consistent with `leaf_block.rs` being a green orphan beside the live π-table path. If gourdon's dispatch routes through LeafBlock, show `pre_marathon_gate` output through the new path.

### What the telemetry actually proves (and doesn't)

- It **proves the mechanism is viable on this silicon** — primecount's D closes in 0.090s with segment-local popcount. It does **not** prove Titan's P1 achieves comparable density. That's a measurement, not an inference.
- **Budget inference:** 0.262s total cannot physically sieve the full v-range up to x/y ≈ 3.5×10⁸ *and* process the leaf cells. primecount is physically touching only the hard band — the z-split's (y, z] restriction — and delegating the rest analytically. Implication: **Front 2 isn't a follow-on optimization; it's part of P1.** Without it, Titan's block engine spends its win re-sieving v-territory the opponent never touches. (Inferred from the time budget — confirm in source. And identify what `k = 8` gates; it's a dial in the opponent's engine you haven't mapped.)
- **The missing instrument is Titan's own term telemetry.** You have K0 chops — put Titan's 6.01s on the same term table as primecount's 0.262s. The 22× lives somewhere; until it's allocated across Titan's terms, the ladder is still aimed by vibes.

### One design gift the census enables: hybrid dispatch

Blocks whose census cell-count falls below a crossover threshold should **never be built** — those cells fall back to the global π/M tables. Dense blocks amortize 10⁵⁺ cells over one build. The crossover is per-scale, derived from the histogram, stored in the scale-indexed dispatch. This kills the sparse-tail build problem without you ever having to analytically resolve the v-density profile — the census decides it empirically.

### Phase 21 exit criteria — amend the gate

- **E1 Correctness:** 5-point differential through the LeafBlock dispatch path at 10¹²+; new 1T-vs-8T bit-exact test of the v-range partition.
- **E2 Performance:** R1 race at 10¹²/10¹³/10¹⁴ — pinned, interleaved A/B, ≥30 reps, median + IQR, thermal interlock.
- **E3 Attribution:** K0 rerun; gate on **|Σ attribution − wall-clock cycles| < 5%** at every scale (this also finally closes the standing 118.17-attributed vs ~67-wall discrepancy); π-lookup, M-lookup, and DRAM-variance each ≤ ~6 cy/cell; total ≤ 35 cy/cell at 10¹⁴.
- **E4 Amortization:** cells/block histogram, blocks built vs touched, build cy/cell, crossover stats.
- **E5 Honesty:** re-mark Phase 21 — 12/12 becomes something like 5 PASS / 7 OWED until E1–E4 produce numbers. Phase 4's 8 debts stay visible.

And holding my line from last review: even with E3 landing at ~35 cy/cell, mechanism (~4×) × z-split (~3–4× if it behaves like the opponent's) × fusion (~1.3×) is *multiplicative headroom on paper* — conceivably 0.3–0.5s at 10¹⁴, i.e., near-parity territory. Conceivably. Every factor is a measurement to be earned, and no verdict line gets written before the race table exists.

**The next deliverable isn't code — it's three pasted command outputs:** `pre_marathon_gate`, `k0_attribution`, `race_session`. That's the entire difference between "implemented" and "done."

Want the v-range partition + arena builder + census-weighted chunker as a concrete patch plan, or the amended Phase 21 gate JSON first? I'd do the gate JSON — it's an hour, and it makes the scoreboard stop lying on your behalf.
