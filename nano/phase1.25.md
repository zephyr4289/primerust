Yes — it's worse, and I'll quantify exactly how much before I hand you the fix, because the receipts you *did* paste contain enough arithmetic to convict the implementation without running anything new.

---

### §1. The ledger, read without mercy

| @10¹⁴, 8T | Phase 20 | Phase 22 gate | Phase 23 truth | **Phase 24** |
|---|---|---|---|---|
| Substrate | 6.015s | 23.932s | 14.202s | **62.633s** |
| vs primecount 0.2285s | 26× behind | 105× | 62× | **274× behind** |

Four consecutive sessions, monotone engine decay: **6.02 → 23.9 → 14.2 → 62.6.** Meanwhile the scoreboard went 96.0 → 96.4 → 96.6 → **96.8%.** The instrument is now anti-correlated with reality. Phase 24 failed its own exit gate P24-6 (≤ 2.0s, got 62.6s — a **31× miss**) and was marked 12/12 PASS. That's not a debt in a bowtie anymore; that's a forged receipt.

The claimed deltas don't close: Φ_c flat (−6.76s) + Lehmer de-inlined (−9.73s) should put the gate at **~7.4s**. Measured 62.6s. There is **+55.2s of unexplained new cost**, and only one component is big enough to absorb it: **D**. D went 6.73s → ~61.9s, i.e. the walker went from 152.9 cy/cell to **~1,412 cy/cell — a 9.2× per-cell regression.**

### §2. The fingerprint identifies the disease

Two signatures in your own numbers:

1. **Uniform ~2.5× multiplier, same growth curve** (7.77/7.93/7.76 per decade, vs Phase 22's 6.9/8.0/8.3). A constant per-cell cost, not a new asymptotic term — this is a *memory-system* regression, not an algorithmic one.
2. **The multiplier grows with scale**: 2.33× at 10¹² → 2.63× at 10¹⁴ → 2.44× at 10¹⁵. Now do the geometry: a *global prebuilt block table* — which your own Phase 22 text advertised as "statically pre-allocated" block storage — spans v ∈ [√x, x/y] at 65,536 integers/block:

| Scale | Blocks | Block table size | Cache class | Per-cell cost |
|---|---|---|---|---|
| 10¹² | ~255 | 4.2 MB | L3 | moderate |
| 10¹³ | ~1,200 | 19 MB | L3/DRAM | bad |
| 10¹⁴ | ~5,250 | **86 MB** | DRAM+TLB | **~1,200–1,500 cy** ✓ |
| 10¹⁵ | ~24,400 | 400 MB | DRAM | worse |

A block table that is L3-resident at 10¹² and 400 MB at 10¹⁵ produces *exactly* a mildly-scale-growing constant multiplier on top of the old random-access growth curve. **1,412 cy/cell ≈ binary search over block headers (~200 cy, L2) + 3–4 DRAM misses across `odd`/`pi_w`/`m_w`/`mu2` (~600–900 cy) + TLB walks on 86 MB of random access.** The arithmetic closes.

**The crime:** Layout B was wired as a *random-access data structure*. A 16.1 KiB block is only "L1-resident" if you are standing inside it when you query. Accessed randomly per cell through an 86 MB table, it is simply a π-table that is **86× fatter and 10× slower** than the one it replaced. The 24 KiB Layout A had the same disease (which is why Phase 22 regressed); Layout B shrank the block and kept the disease. Alternate suspect (H2: on-demand build with LRU thrash, ~7–15M rebuilds × ~150k cy) fits the magnitude too — the discriminator is three counters, below.

### §3. Why the opponent's D is 0.090s — the convergence you already measured

0.090s × 17.7 Gcy/s = **1.59 Gcy.** Sieving [√x, x/y] ≈ 3.5×10⁸ slots at ~4.5 cy/slot ≈ **1.6 Gcy.** The match is not a coincidence: primecount's D is *physically a segment sieve + popcount pass over the v-axis*, with the leaf algebra riding on the popcount prefixes at ~1–2 cy per leaf. A per-cell random walk — at any cy/cell — can never reach a streaming pass. Conversely, my block design **is** that pass, done with transient blocks. The design was right; the wiring inverted its entire reason to exist. Also note: π(z) − π(y) ≈ 21,761 primes with e_max ≈ √x/p ≈ 35 gives only ~550k *direct* hard-leaf cells — meaning most of Titan's 776M-cell surface is leaves the opponent never physically walks (they live in AC/B/Σ, analytically). Your 24-D "formulated" but did not wire this: D was supposed to shrink ~6×; it grew 9×.

### §4. Phase 25 — the fix, concrete and complete

**Invariants as code, not prose** — these make both failure modes structurally impossible:

```rust
// INVARIANT 1: No global block table EVER exists. Blocks are stack-arena
//              transient: build -> serve every cell in it -> discard.
// INVARIANT 2: Each block built AT MOST once per run (debug_assert on counter).
// INVARIANT 3: Sparse region (v > v_star) NEVER touches blocks: it uses the
//              global PiTable/Mertens path — random access is CORRECT there,
//              because cells/block < ~1,000 (build 250k cy vs ~250 cy/cell DRAM).

pub struct Arena25 {                 // per-thread, ~72 KiB, zero heap traffic
    blk: LeafBlockB,                 // 16.4 KiB working block (Layout B, as spec'd)
    pending: Vec<PendingCell>,       // 4 B entries: { j: u16, e: u16 }
    base: Vec<u64>,                  // 2,130 primes ≤ √(x/y) — 17 KiB, L2-streamed
}

pub struct SweepPlan {               // census-derived, per scale
    v_star: u64,                     // hybrid crossover from cells/block histogram
    chunks: Vec<Chunk>,              // census-weighted equal-cell v-ranges
}

fn sweep_chunk(c: &Chunk, arena: &mut Arena25, acc: &mut i128) {
    // PASS 1 — enumerate ONCE per cursor: 2 magic divisions (umulh) bound
    //          the e-window of every hard-band cursor inside this chunk;
    //          push (j, e) per cell, v-descending per cursor. ~15 cy/cell.
    // PASS 2 — block pipeline, v-descending:
    //          build (≤250k cy) -> replay every pending cell in the block
    //          via b.pi_at(v) / b.m_at(v) (~5–10 cy, L1D by construction).
}
```

**Build pipeline (the 250k cy budget):** presieve patterns for p ≤ 31 that write 2-bit μ-codes directly (`memcpy` of 64 B patterns encoding composite / p²-squarefree-zero / ω-parity-flip); marking loop for p ∈ (31, √v_hi] fusing composite strikes **and** parity flips in one pass; NEON `cnt`+prefix for `pi_w`; 256-byte LUT decode of `mu2` → `m_q` → `m_w`. Byte budget: 4 KiB bits + 8 KiB μ2 + 1 + 1 + 2 KiB prefixes = **16.4 KiB**, plus 4 KiB scratch — one block resident, nothing else.

**Parallelization:** 64–256 census-weighted chunks, `fetch_add` stealing, per-thread private arena. Chunk-local cursor state is *recomputed* at entry by magic division (M(e_entry) from a 130 KiB L2-resident e-domain Mertens table) — this is why the partition is bit-exact by construction, no locks.

**Cost model @10¹⁴, 8T:** builds 5,250 × 250k = 1.31 Gcy (74 ms); enumeration + replay ~2.5–9 Gcy (census-dependent); sparse region < 0.1 Gcy. **D ≤ 0.5s Tier-1. Total ~1.2s.** Gate: frozen probe ≤ 1.6s.

### §5. Tier-2 — the anomaly move your own census unlocks

Your census: 776M cells, **15.0M distinct v, 51.8:1 sharing.** If the leaf term is affine in the looked-up values — `term(j,e) = c(j,e)·F(v) + d(j,e)`, F ∈ {π, M} at the cell's own v, with e-domain parts served by the small table — then per-v pre-aggregation collapses D to **Σ over 15M distinct v** of `F(v)·Σc(v) + Σd(v)`: ~15M × 20 cy ≈ 0.3 Gcy + builds ≈ **D ≈ 0.10–0.15s. The opponent's number.** Proof protocol, not trust: run Tier-1 and Tier-2 side by side, assert **i128 equality at 10¹²/10¹³/10¹⁴**, then delete Tier-1's replay. This is almost certainly the algebra behind the opponent's "POPCNT64" D — we re-derive it, then beat it on the sieve itself (fused count, zero prime materialization — Phase 26) and on 2+6 scheduling.

### §6. Exit criteria — non-negotiable

- **P25-0 (before any code, half a day):** run `term_breakdown` on the current build (the Phase 23 instrument exists and was *not run* for this report — that omission is why 55 seconds shipped invisibly); add counters `blocks_built`, `block_lookups`, `cells_per_build` (H1 → builds ≈ 5,250; H2 → millions); isolated receipt for the Φ_c "3–5 ms" claim (currently unevidenced); the census table `cells_hard(β)` still owed from P24-4.
- **P25-1:** Tier-1 bit-exact A/B vs the current walker at 10¹²/10¹³; 1T ≡ 8T on the chunk partition; D ≤ 0.6s; probe ≤ 1.6s.
- **P25-2:** Tier-2 A/B equality at three scales; D ≤ 0.2s; probe ≤ 1.0s.
- **P25-3:** Re-mark **Phase 24: FAIL** (it failed its own gate by 31×). New law: no phase closes without the frozen-probe receipt attached; any probe regression > 10% auto-fails the phase regardless of criteria count; completion% stays suppressed until the ledger (Phase 4's 8 debts + re-marks + missing phases 12/17) is visible.

Standing projection, honestly held: post-25 Tier-2 lands ~0.75–0.9s at 10¹⁴ (S₂ 0.41s becomes the next target), i.e. **3–4× behind — with parity reachable via Phase 26 (fused S₂ + setup trim) but not promised.** At 10¹⁸ the v-sieve floor (~1.6×10¹¹ slots) plus S₂ (~2.2×10¹² slots) at fused ~2.5 cy/slot gives a hard physical floor of ~5–6 minutes on this silicon — sub-15-minute π(10¹⁸) is back inside the window only if every tier lands.

The correctness path never broke — bit-exact through a 10× slowdown, which tells us this is a pure mechanical disease, the curable kind. Ship P25-0's four receipts, then the Tier-1 patch; I'll write the Tier-2 coefficient-collapse derivation against your exact 5-term identity the moment the census semantics (cells vs runs vs distinct-v) are pinned by the counters.
