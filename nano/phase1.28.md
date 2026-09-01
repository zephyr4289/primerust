Read it — and this time the news is real, so I'll open with the audit result before the convictions: **both receipts survive forensic interrogation.** First phase since Phase 20 where that's true. Then I'll show you the discovery sitting inside your own census table that neither of us put there — it rewrites the Phase 27 plan.

---

### §1. What I verified (and it held)

| Check | Anchor | Receipt | Verdict |
|---|---|---|---|
| π(10⁴)=1,229, p₁₂₂₉=9,973; π(10³)=168, p₁₆₈=997 | textbook | Receipt #4 header | ✓ real |
| Cells at 10¹² = 41,438,286 | ×4.33/decade law → 776M/18.7 | ✓ exact |
| Blocks = (x/y − √x)/65,536 | 1,511 predicted | 1,510 | ✓ (run at **α=1**, note) |
| Blocks column, all 24 grid points | (x/y−√x)/65,536 | 32,723 / 16,285 / 5,254 / 3,954 | ✓ exact |
| Hard primes (α=6.085, β=2) = 21,794 | my independent π-interpolation: ~21,800±200 | ✓ |
| **Hard-cell law:** `cells ≈ 0.82·(x/y)·Σ_{p∈(y,z]} 1/p` | α=1: 79.7M→65.5M (f=0.82); α=2: 37.5M→30.8M (f=0.82); α=6.085: 15.4M→13.7M | ✓ **consistent across the grid** |
| Builds/block = 1.00, 27,442 cells/block = 41.44M/1,510 | | ✓ — **kills the Phase 24 thrash hypothesis (H2) for the arena path** |
| 1T D at 10¹²: 0.5405s × 2.208 GHz ÷ 41.44M | | **28.8 cy/cell — the Tier-1 cost model, landed** |

The census's hard cells are structurally the (p, m) leaf pairs of the true z-split — Σ over the band of cofactor counts — not a walker-shaped fiction. The arena is build-once, transient, L1-local, ~29 cy/cell single-thread. That's a 5.3× kernel improvement over the 152.9 cy/cell state, **measured, in the instrument.** The instrument suite is 2/4 recovered. Credited, loudly.

### §2. Four convictions

**1. "CONSERVATION LAW CERTIFIED" is the invariant, hollowed.** I specified `hard(α,β) + delegated(α,β) = total(α)` per dial. The banner certifies "non-zero and bounded" — trivia. There is no delegated column, no identity, and therefore **the 2-D argmin optimizes half a cost function.**

**2. The α=8 "optimum" is the grid edge, not an optimum.** Your own estimator deltas: −0.271, −0.090, −0.045, −0.045, −0.021 — monotonically decreasing in α, asymptoting to the S₂ constant. A model with no rising term has no interior minimum; sweep α=12 and the "optimum" moves to 12. The rising term — delegated analytical surface + growing π(y) machinery — is exactly what's unpriced. Also: S₂ is mispriced as α-independent when its sweep horizon [√x, x/y] shrinks 1/α; and the 4.6% margin between α=8 and α=6.085 is inside this phone's thermal noise band. Dial decisions at 4.6% margin are numerology with extra steps.

**3. "Matching Walisch's measured 21,761" — Walisch measured nothing.** *I* estimated 21,761 in Phase 23 from π(z)−π(y) interpolations, and it echoed back two phases later as "opponent telemetry." Your census's 21,794 is probably the correct number and my estimate the drift. I'll own my error, because it proves the law one more time: **unverified constants propagate as measurements** — the same mechanism that created the 46,416 three-way collision. Related: "Total Primes in Base Array: 664,640" vs the most famous anchor in existence, π(10⁷) = 664,579 (+61). Almost certainly a benign sieve-limit overshoot (~10⁷+1000) — but on this project, a 61-drift on a known anchor gets a one-line explanation, not silence.

**4. Receipts #1 and #3: third consecutive absence.** Receipt #4 proves the arena works *in `arena_counters.rs`*. Nothing proves the production engine dispatches through it, and nothing applies the studied dial. The last measured production state remains 62.6s. The scoreboard ticked 96.9 → 97.1% anyway. One command, three phases, not run. That's no longer an omission; it's the pattern.

### §3. The discovery in your table: the anti-correlation

Divide your own columns — cells per block, and the build amortization at 250k cy/build:

| Dial | Hard cells | Blocks | **cells/block** | **build cy/cell** |
|---|---|---|---|---|
| un-split, α=1 (Phase 20 walker) | 776M | 32,723 | 23,717 | **10.5** |
| α=1, β=1.5 | 65.5M | 32,723 | 2,003 | 125 |
| α=6.085, β=2 (opponent dial) | 13.7M | 5,250 | 2,615 | **96** |
| α=8, β=1.5 | 6.9M | 3,957 | 1,741 | **144** |

**The α-collapse and the LeafBlock engine are anti-correlated weapons.** The retune that shrinks the walked surface 56× also dilutes its density 9× — because the hard band spreads its (p, m) leaves across the *full* v-span [y, x/y] regardless of β. Above α ≈ 4, a 250k-cycle block build costs more than the DRAM lookups it saves: **the block economics invert.** This also explains, retroactively, why the opponent's structure is what it is: at α=6.085 the hard surface is *sparse* (~1 cell per 26 integers) — random access into global tables is the *correct* engine there, and segment-local popcount serves the sieve, not the leaf lookups. Blocks want no z-split (density 23,717/block); the z-split wants no blocks (density ~2,000/block). Your census just proved these are *alternative* structures, not composable ones — and the estimator that ignores this cannot pick between them.

**The synthesis — density dispatch, which your census already instruments:** per v-block, `census_cells(block) > crossover ⇒ build; else global-table walk`. At the opponent dial, the (p·m) divisor concentration means a minority of v-blocks holds most of the 13.7M cells — the histogram (the same instrument that produced 51.8:1 sharing) decides block-by-block. The dial grid and the density dispatch compose; neither alone does.

### §4. A live correctness hole the 1T≡8T receipt cannot see

Layout B packs μ at **odd residues only**. π is parity-safe (π(2k) = π(2k−1), no even primes > 2). **M is not: M(6) = −1 ≠ M(5) = −2, because μ(6) = +1.** If the walker's M-argument is ever even — v = ⌊x/(p·e)⌋ certainly can be — then `m_at` silently returns M(v−1) and D is wrong *deterministically*, identically on 1 and 8 threads. The equivalence receipt proves determinism, not correctness. Two probes, one afternoon:

```rust
// P27-PARITY-1: exhaustive — 65k comparisons, catches it instantly
for v in blk.v_lo..=blk.v_hi {
    assert_eq!(blk.m_at(v) as i64, global_mertens.m(v) as i64, "v={v}");
}
// P27-PARITY-2: external anchor — full engine through the arena dispatch
assert_eq!(pi_via_arena(10^12), 37_607_912_018);
```

If the hole is real, **Layout C** fixes it at the same budget: re-index per *integer*, 512 words = 32,768 integers/block — bits 4,096 B + μ₂ 2 bits/int 8,192 B + π_w 1,024 B + m_w 1,024 B + m_q 2,048 B = **16,416 B, identical 16.1 KiB**, both parities exact, at the cost of 2× block count — which §3 says you only pay in dense regions anyway, where amortization is 10–20×.

### §5. Phase 27 — concrete

- **P27-1 (the command, fourth time of asking):** frozen probe on HEAD. Expected: ~62.6s if unwired; ≤2s if the arena + a dial are dispatched. This single number also delivers P27-PARITY-2's anchor. Wire the dispatch:
```rust
let dial = scale_dispatch.dial(x);              // from census v3, interior optimum
let d = if dial.split { d_hard_sparse(dial, tables)   // global-table walk, Tier-2 pre-agg
                        .merge(dense_blocks(dial, histogram, arena)) } // build-if-dense only
        else   { arena.sweep_unsplit(x, chunks) };    // 23.7k cells/block, 8.4 cy build amortization
```
- **P27-2 (census v3):** add the delegated column + `assert_eq!(hard + delegated, total_per_dial(α))`; add per-block cell histogram; complete the cost model `T(α,β) = setup(α) + S₂(α) + builds_above_crossover·t_b + hard·t_cell + delegated·t_flat`; sweep α ∈ [1, 12]. **Exit: the optimum goes interior.** If it still sits on an edge, the model is still missing a term — that's the falsifier.
- **P27-3:** parity probes; Layout C if they fail.
- **P27-4:** pinning statement on the 1T baseline + per-core cell counters. At 2.28× with 1T-on-big, the six A55s are contributing ~5% of a big core each — either the chunks aren't census-weighted or 1T ran unpinned; Titan's own historical 8T/1T at 10¹² was 3.12×, and the 3.5× pool moat is not demonstrated by this harness.
- **Re-mark Phase 26: ~7 PASS / 5 OWED** (census real, counters real, coherence real; owed: conservation identity, delegated column, receipts #1/#3, production wiring, external correctness anchor). 97.1% while the engine's last measured word is 62.6s remains theater.

### §6. Honest projection

Both weapon paths now have measured or derivable cost floors at 10¹⁴: dense/un-split ~0.4–1.0s (Tier-1–2 + builds), sparse/retuned ~0.25–0.5s (13.7M cells, global lookups + flat terms + S₂-at-α). **Total ~0.8–1.5s → 4–6.5× behind** — parity stays a Phase 28 problem (S₂ fusion + setup trim), not a promise. At 10¹⁸, the sparse path scales structurally: hard surface ≈ (x/y)·Σ1/p ≈ 9G leaves — but S₂ at the α=6 horizon (~1.6×10¹¹ slots, ~200s unfused) becomes the capstone, exactly as forecast. Sub-15-minute π(10¹⁸) lives or dies in the S₂ fusion, not in D.

The census did its job — it found the thing the roadmap didn't contain. Now run the probe: one command, four phases owed, and it decides whether Phase 27 engineers a *wired* engine or another green orphan.
