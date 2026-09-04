Yes — every one of them is solvable. Ranked by confidence:

- **P1a (u32 width):** solved mechanically, highest confidence. It's templating, not math.
- **P1c (serial C1):** trivially solvable, free money.
- **P1b (per-visit overhead):** solvable — it's deleted-work restructuring, not tuning.
- **P1d (sieve):** solvable, bounded, small.
- **P2 (region fusion):** solvable but refactor-shaped. Two levels — cheap hoist, then true fusion.
- **P3 (remaining deleted work):** partially — PhiCache/FactorTableD are buildable, but pole-constrained (see Law 1 below).
- **P5 (personality split):** solvable and genuinely novel — this is your only structural edge, because his binary is fixed and silicon-agnostic.
- **P4 (noise):** not solvable, only *disciplined* — but it's the gate on claiming anything else.

One framing fix before the plan: **"obliterate at 10⁶–10¹⁹" isn't the right scoreboard.** Below ~10¹¹ primecount doesn't even run Gourdon — it routes to Lehmer/sieve paths finishing in milliseconds; there's no headroom there, it's a different contest. Your battlefield is **10¹²–10¹⁸**, where the Gourdon terms dominate and your two compounding advantages (u32 leaf structure + asymmetric core mapping) persist across the whole range. 10¹⁹ is a *memory* fight, not a compute fight — gate it on measured RSS scaling, don't promise it.

Also, notice something in your own data: primecount's B(8T)=0.69s → **β = 5.5 thread-seconds**, D(8T)=1.13s → **δ = 9.0 thread-seconds**. Your fit constants *are his constants*. Your B/D work is already at exact thread-second parity. That means every future B/D gain must come from **deleting work or exploiting asymmetry — never from threading**. Your P0 instinct is correct and generalizes:

---

## Two laws that govern the whole plan

**Law 1 (pole law):** B+D wall = min over t_B of max(β/t_B, δ/(8−t_B)). At β=5.5, δ=9.0 the pole is 1.81s at t_B=3 — which you already hit. Corollary: **a cut to the non-pole side buys nothing.** Any D-side optimization (FactorTableD, sieve quality in D) is worthless until δ/5 < β/3 *and* you also cut β. After every phase, refit β and δ as thread-seconds and recompute. After Phase 7 they split per core class.

**Law 2 (exactness law):** everything is integer arithmetic. Every refactor must produce **bit-identical term sums** vs the previous build. This gives you a zero-cost oracle at every step — no correctness risk ever justifies itself.

---

## Phase 0 — The rig (half a day, before touching perf code)

- Interleaved A/B harness: coin-flip run order per pair, ≥30 pairs, fixed cooldown between pairs, report p50 + MAD per binary *and per term*. Discard first 5 pairs.
- Record silicon facts empirically: `sysconf(_SC_LEVEL*_CACHE_SIZE)` per core, topology + capacities from `/sys/devices/system/cpu/`, current freq per core logged with every run (detect clamping drift).
- Build primecount locally with `-O3 -mcpu=native` as an **oracle, not the opponent**. It tells you how much of the remaining gap is his composition vs his generic-binary handicap. The shipped `-O3` binary is the opponent.
- Acceptance gate: run-to-run MAD < 0.05s. **Nothing below Phase 1 is provable until this passes.** His 2.73–3.6s swing means his "2.96" is cold-gifted; fair protocol probably *raises* his median — the win gets easier to claim, not harder, but only if you measure it.

## Phase 1 — Width specialization (P1a)

- Template the kernels on prime type: `template <typename PrimeT>` → u32 stream when y < 2³² (true for the entire realistic range). Prime stream 2GB→1GB, all index/magic tables shrink.
- Second, independent move: **bucket the leaf ranges by quotient width**. Wherever q = x/(p_b·p_m) provably < 2³² by range analysis, instantiate a u32-quotient leaf kernel. On the A55's in-order pipe, 32×32→64 (`UMULL`) vs `UMULH` is a ~3–4× MAC-slot difference; on A78 it's ~free. So this fix is really an **A55-side fix that later feeds Phase 7 routing**.
- Struct-of-arrays layout: `primes32[]` and `magic[]` as separate sequential streams.
- Verify in disasm: zero `__udivdi3`/libgcc calls; `UMULH` only in the u64 bucket.
- **Expected: −0.10–0.20s (AC).** Gate: dual-run shadow mode (u32 + u64 kernels on same segments, diff must be 0), plus π(10¹⁶)=279238341033925, π(10¹⁷)=262357157654233.

## Phase 2 — Parallelize C1 (P1c)

- Parallel over the ~77 b's, per-thread partials, per-thread recursion stacks, tree-combine at the join. Fold it into the AC region — no new region, no new pool.
- Pure integer sums → bit-identical by associativity. Zero math risk.
- **Expected: −0.08–0.12s.** Verify: C1 wall 0.14s → ≤0.05s.

## Phase 3 — Delete per-visit overhead (P1b)

- **Kill `isqrt64` from the hot loop entirely.** The m-limit m ≤ π(√(x/p_b)) is monotone non-increasing in b. Walk it backward with a pointer + table probes, amortized O(1) per b, no sqrt, no float path. This is the same deleted-work pattern he uses.
- **Replace `QuotientWindow::pi()` with per-segment `segPi[]`.** Bucket quotients (e.g. `63−clz64(q)` + low bits → ~1–4K buckets per segment), fill once per segment, keep ≤16KB so it's L1-resident. Leaf does one indexed u32 load instead of sub/shift/2 loads/popcnt/add.
- Hoist all per-b invariants (magic, base pi, bounds) out of the leaf loop; branchless min/max.
- **Expected: −0.08–0.15s (AC).** Verify: instructions-per-leaf via simpleperf, AC median.

## Phase 4 — Sieve quality (P1d)

- Persistent per-thread segmented sieve: fixed arena, `init(low, high)` re-sieves in place, zero alloc per window. Wheel-30 byte pattern, unrolled cross-off.
- Segment size parameterized by executing core's L1D — this is Phase 7 plumbing installed now.
- **Expected: −0.03–0.05s.** Smallest, so it's last of the cheap set, but it removes an allocation path you'll want gone before fusion anyway.

## Phase 5 — Pipeline (P2), in two cuts

- **5a (cheap):** build all shared read-only tables once before the first region (sieve, FastDiv, base primes, mu), persistent per-thread scratch. Keep the 3 FFI regions — FFI itself is proven free (1.32 vs 1.33). Before assuming where the 0.1–0.2s lives, **instrument region-enter → first-task-dispatch**: my suspicion is it's table *construction compute* ×3 (~50ms each), not barriers or pool wakeups, which are μs-scale. Measure, don't assume.
- **5b (true fusion):** one native region, typed task queue (AC-segment, C1-b, B-batch, D-batch, combine) over one persistent pool. B/D stop being FFI regions and become tasks. Keep the (3,5) *work assignment semantics* identical — only the scheduling substrate changes. Preserve whatever pipelining dependency B has on D as a task-graph edge.
- **Expected: −0.05–0.10s each cut.** Risk: medium — this is the one refactor where a scheduling bug hides. Law 2 is your net.

## Phase 6 — Deleted-work parity (P3 remainder), pole-aware

- **PhiCache:** per-thread memo for the φ recursion (C1 + B leaf paths), his structure: per-m-level arrays, entry-gated, MB-capped. Instrument φ call counts first; only build it if projected hit rate justifies the memory.
- **FactorTableD-style 2-bit table** over the wheel range fused into D's candidate scan — **but check Law 1 first**: at δ=9.0, D-side cuts are currently dead weight (B is the pole at t_B=3). Bank them only if you also cut β, or shift the split.
- **S2_approx-style balancers:** analytic per-(segment,b) leaf-count prediction → cost-weighted dispatch. Mostly subsumed by Phase 7, build it there.
- **Expected: −0.05–0.10s, only where pole-side.** Refit β, δ after.

## Phase 7 — Personality split (P5) — the differentiator

This is the phase that turns parity into a win, and it's the one nobody has built:

- Measure per-core-class throughput of every kernel: β_A78, β_A55, δ_A78, δ_A55 (pinned single-thread runs).
- Pool workers pinned by class: A78s (OoO, 64KB L1) get **divergent/recursive work** — C1 descents, clustered leaves, u64-quotient buckets, bound walks. A55s (in-order, 32KB L1) get **straight-line streaming** — u32 leaf loops (post-Phase-1 these are now *relatively* stronger on A55 — the two phases stack), sieving, table builds.
- Size classes: 32KB-bounded segments on A55, 64KB on A78. **His binary cannot do this — his L1D default is wrong on 6 of his 8 cores and his segments thrash.** That's structural headroom that only exists for you.
- Static weighted assignment: solve min-makespan with per-class speeds (weighted LPT), dynamic fallback queue with class-aware weights. A55s get larger chunks (sync amortization), A78s smaller (balance).
- The (3,5) split stops being a scalar and becomes a per-core assignment — this is the *only* legitimate reason to touch it. Refit under Law 1 with the two-column β/δ ledger.
- **Expected: −0.15–0.30s.** Risk: medium; keep the symmetric path behind a flag as fallback.

## Phase 8 — Exotics and the scale campaign

Only after the above lands:

- NEON-ize the u32 magic division (VMULL + VSHRN pairs, 4 quotients per instruction pair); keep loads scalar — gathers are poison on A55.
- **Personality-dependent leaf partitioning:** different b-split per core class — mathematically exact if the partition is disjoint and complete (Law 2 holds). High risk, high reward.
- **RSS scaling study** 10¹⁶→10¹⁷→10¹⁸ before any 10¹⁹ claim. And here's the structural good news for the scale story: for all x ≤ 2⁶⁴, the quotients ≥ √x fit u32 — **the u32 leaf path isn't a 10¹⁶ hack, it's valid across the entire range.** Your advantages compound with scale; his fixed binary degrades (64KB assumption, symmetric worldview). That's where "scale wins" is actually true.

---

## Ledger (median, additive assumption — gains partially overlap, treat as optimistic)

| Phase | Δ | Titan cum. |
|---|---|---|
| start | — | 3.35 |
| 1 width | −0.15 | 3.20 |
| 2 C1 | −0.10 | 3.10 |
| 3 leaf overhead | −0.12 | 2.98 |
| 4 sieve | −0.04 | 2.94 |
| 5 pipeline | −0.12 | 2.82 |
| 6 deleted work | −0.07 | 2.75 |
| 7 personality | −0.20 | 2.55 |
| 8 exotics | −0.10–0.20 | 2.35–2.45 |

Opponent under fair protocol: ~3.0–3.1. **Realistic end state: 15–20% win, stretch 25%.** Not obliteration — you're fighting the same math with the same memory system — but you'd be beating the best-in-class implementation on silicon it was never tuned for, which *is* the actual win condition.

## Guardrails

- (3,5) split frozen until Phase 7, then re-derived from per-class β/δ — never hand-tuned again.
- Every phase ships behind a flag; term sums bit-identical vs previous build or it doesn't ship.
- Claim protocol: interleaved p50, gap > 2×MAD, or you don't claim it. "Obliteration" claims die here permanently.
- Log everything (git SHA, config, medians). You will otherwise re-litigate old results.

**Kickoff: Phase 0 + Phase 1 in one session** — the rig takes half a day and Phase 1 is templating you can validate immediately with the shadow-mode diff. Start there, bring me the per-term medians, and we go deep on whichever phase the numbers say is next.
