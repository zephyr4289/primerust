Receipts received. And credit first, because two of my demands actually landed: the K0 total now **reconciles with wall-clock** (6.7188s × 8T × 2.208 GHz ÷ 776M cells = 152.9 cy/cell — exact), and the 5-point differential suite plus the cross-engine bit-exact check at 10¹⁵ are green. Instrument integrity is improving.

Now the part you're not going to enjoy: **these receipts don't support the phase narrative. They document a 2–4× engine regression that the report never mentions, and the phase's headline claims are contradicted by its own K0 table.** "Unvarnished receipts, varnished findings."

### 1. What the receipts actually say

| Comparable pair | Prior session | This session | Factor |
|---|---|---|---|
| Substrate 8T, 10¹² | 0.2439s | 0.436s | **1.8× slower** |
| Substrate 8T, 10¹³ | 1.1872s | 2.999s | **2.5× slower** |
| Substrate 8T, 10¹⁴ | 6.0147s | 23.932s | **4.0× slower** |
| 10¹⁵ marathon | 105.58s | 199.00s | **1.9× slower** |
| primecount 8T, 10¹⁴ | 0.2748s | 0.2285s | **1.2× faster** |

Gap at 10¹⁴ (8T): **21.9× behind → 62× behind.** While primecount got *faster*, on the same device, in the same session. The sieve binaries on both sides drifted ~±20% (thermal noise — this phone's variance band), but 2–4× is far outside it, and it's unilateral to Titan.

Three confounds make the exact factor per engine unprovable — each an instrument failure in itself:

- **The race switched engines without note.** Header now says "TITAN LEHMER"; the prior session's engine was unlabeled. So the 1T regressions (23.42→48.73s at 10¹⁴) may be Lehmer-vs-substrate, not regression. But the *substrate* numbers from `pre_marathon_gate` have no such excuse.
- **`pre_marathon_gate` and `race_session` disagree by 1.7× at 10¹⁴** (23.93s vs 14.17s), and the disagreement grows with scale. The gate output shows an inline cross-engine Lehmer check at 10¹⁵ — if per-row times include hidden verification runs, the rows are contaminated as engine timings. Verification cost must be timed separately, or the instrument is measuring two engines stapled together.
- **No frozen baseline table exists.** Three sessions, three shifting yardsticks.

### 2. The attribution smoking gun

Lay Phase 18's K0 next to Phase 22's:

| Component | Phase 18 | Phase 22 |
|---|---|---|
| π lookup | 32.0 | 32.0 |
| M lookup | 30.0 | 30.0 |
| Magic division | 6.0 | 6.0 |
| Branch/state | 18.0 | 18.0 |
| "DRAM variance" | 32.2 | **66.9** |

Four components **identical to the decimal across sessions and a code change** — those aren't measurements, they're constants in `k0_attribution.rs`, and "DRAM variance" is the residual bucket that absorbs whatever the wall clock says. You fixed the *total* and kept the fiction in the *components*.

And here's the damning part: §1 of your own report claims "≈4–5 cycle queries" and "eliminating per-cell DRAM queries for M(u)". If the LeafBlock were live in the attributed path, the π and M components would have collapsed. **They didn't twitch.** The phase's central mechanism is invisible in the phase's own attribution. Meanwhile DRAM variance *doubled* — the signature of paying new costs (block builds, streaming, contention) without retiring the old lookups. The best-fit reading: the walker now carries block-path overhead on top of the unchanged global-table lookups.

### 3. Where the time actually went — and the receipts are blind there

Substrate at 10¹⁴: 23.93s total, walker 6.72s = **28%**. Prior structure: walker ~86%, non-walker ~0.8s. Now non-walker = **~17.2s — a ~20× explosion — and no receipt, section, or sentence in this report acknowledges it.** The regression lives precisely where your instruments don't look. Candidates to discriminate, in order of fit to the K0 signature:

- **H1:** LeafBlock engaged additively (build/stream overhead added, old lookup path not replaced).
- **H2:** Dual-path equivalence runs executing both engines in production (the mt-equivalence test logic leaking into the dispatch).
- **H3:** Cross-engine verification inlined in per-row timings.
- **H4:** Table builds grew (Mertens/π structures rebuilt larger to feed blocks).
- **H5:** Allocation or lock on the block path despite the "zero-allocation" claim (per-thread `[u64; 2048]` on the stack is fine — prove the arena actually exists).

For scale: honest block building is *cheap*. ~1,350 blocks cover the v-axis; sieving 3.5×10⁸ integers costs Titan's own sieve ~0.15s. Cells-per-block amortization is enormous (776M cells over at most 1,352 blocks — the census's 51.8:1 sharing guarantees it). If block cost were the 17s, something is rebuilding blocks per-cursor or per-something it shouldn't. The census can't be wrong about this; the implementation can.

### 4. Layout A still can't answer M(u)

The arithmetic is now right (24 KiB ✓) — for π. But `m_prefix: [i16; 2048]` is **per-word** over 131,072 odd residues. π queries survive per-word prefixes because you can popcount within the word. M queries cannot — μ ∈ {−1, 0, +1} per residue needs a **2-bit μ stream**, which costs 32 KiB at this block size and blows the L1D budget. As declared, the struct is mathematically incapable of exact within-word M(u) — which is consistent with the M component sitting at 30.0 cy, untouched. Either the M path never moved to blocks, or it's silently falling back to the global Mertens table. Fix menu: 512-word blocks (4 KiB bits + 8 KiB μ2bit + 1 + 1 KiB prefixes = **14 KiB**, safe, still ~thousands of cells per block), or hybrid dispatch with M served globally below the census crossover. Also: the engine tag still says `config: "32KiB-L1D-SM4450"` on a phase about a 24 KiB layout — cosmetic, but it tells me the config string wasn't touched when the code was.

### 5. The scoreboard has become the anti-instrument

Three sessions: criteria 200 → 212 → 224, completion 96.0 → 96.2 → 96.4% — **monotonically improving while the engine monotonically regresses.** Phase 22 is marked 12/12 PASS on the same page as a 4× slowdown and claims contradicted by its own receipts. Phase 21 was not re-marked per the amendment — it's still 12/12. Phases 12 and 17 are still missing entirely. Honest re-mark for Phase 22 as it stands: ~4 PASS (receipts exist, values bit-exact, Layout A arithmetic, K0 total reconciles) and the rest OWED — including the still-absent E4 (cells/block histogram, blocks built vs touched, build cy/cell).

**Law 0 amendment I'd freeze right now:** every phase closes with a delta against one frozen baseline — `10¹⁴ substrate, 8T, pinned, interleaved, median-of-30` — plus per-engine 1T/8T rows at 10¹²–10¹⁵ labeled by engine. A phase that cannot state its own delta on the fixed probe is not done, whatever the criteria count says.

### What's still genuinely alive

Small-x wins hold (2.28×, 1.34×), the 3.68× vs 2.10× scaling ratio at 10¹³ holds, bit-exactness is intact across every differential — including cross-engine at 10¹⁵. The moats survived this phase. The kernel war did not: it went from 21.9× to 62× behind.

**Next deliverable is not code.** Three outputs: (1) a per-phase timing table at 10¹⁴ — sieve / π-table / Mertens / P₂ / walker / block-sweep, 8T, pinned, one run; (2) build counters — blocks built, blocks touched, builds per block; (3) `git bisect` from the last 6.01s build to HEAD using the frozen probe. That localizes the 17 seconds in an afternoon. Until then, every line of new kernel code is aimed at a target nobody has located.

Find the 17 seconds. Then we talk about Layout B.
