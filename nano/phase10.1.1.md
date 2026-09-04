Critique lands. Scoring it as a whole: 1, 2, 3, 7, 8, 11 — fully correct, accepted as written. 4, 5, 10 — correct, and the fix is bigger than the critique states. 6 — half right (attribution false, but "unproven" is wrong — proof below). 9 — correct, and it's the important one: I banned the word instead of writing the phase. Dispositions, then v2.

## Dispositions

**1. π(10¹⁷) anchor — accepted, and it's the funniest possible bug.** A hand-typed digit in a doc whose entire thesis is Law 2 (machine-check everything). Structural fix: anchors become a generated fixture extracted from the tree constants, CI cross-checked; the doc carries **zero digits forever**. Gates cite the fixture.

**2. Phase 5 double-count — accepted, 5b was air.** Worse than billed: the B/D-side table builds live *inside his FFI calls* and can't be hoisted at all without the port — so even 5a's range was partially fictional. Resolution: 5a keeps only AC-side hoisting, gated on the instrumentation (−0.03–0.06 measured, not claimed). 5b's direct value is ~0 and its real content was Phase 9 wearing a trench coat — fusion presupposes native B/D, which the plan never scoped. Deleted as a perf line, reborn as 9.4.

**3. Phantom edge — accepted.** B‖D already runs zero-sync. The task graph is two independent families plus tables→everything. Simpler than what I wrote, not more machinery.

**4. Law 1 — accepted, fixed now, not in Phase 7.** The symmetric law is false on 2×A78+6×A55; it was harmless only because the freeze was empirical (measured 1.79 vs 1.81 pole is within noise — the uniform model *can't be distinguished* from reality in the current unpinned config, which is why the error was invisible). Rule going forward: **no split is refit until the per-class κ table exists; after that, splits come from enumeration, never hands.** Law 1′ below.

**5. u32 range claim — accepted; direction backwards, prose was a trap.** Correct statement: pair-quotients q = x/(p_b·p_m) ≤ x/x_star², and with the tree's α-curve that stays < 2³² for every representable u64 x. But note the sharper version: *single-prime* quotients x/p_b at 1e16 already exceed 2³², so bucketing must be per-leaf-form range analysis in code — static assert on the α-curve bound plus a debug guard per segment. The assertion ships; the prose doesn't.

**6. isqrt — split verdict.** Attribution retracted: his AC calls isqrt per (segment, b); "same pattern he uses" was false. Meta-lesson adopted as a doc rule: **no prose claims about his code without a source line or measurement — anything unverified is marked UNVERIFIED and cannot gate anything.** But "unproven" is wrong, and the proof is two lines: for integer p_b, m_max(b) = max{m : P[m]² ≤ ⌊x/p_b⌋} = max{m : p_b·P[m]² ≤ x}, because n²·p_b is an integer and n² ≤ a/b ⟺ n²·b ≤ a. The floor semantics dissolve exactly — no off-by-one exists to chase. Walk `while p_b·P[m]² > x: m--`, monotone across b, amortized O(1). Overflow: probe product ≤ x·(1+ε) with ε ~ gap/√ — safe for x ≤ 1e18; if you ever push near 2⁶⁴, make the probe u128. Being *not* his pattern upgrades this from parity-restoration to a deletion he doesn't have.

**7. FFI parenthetical — accepted, deleted.** 1.32 vs 1.33 was two implementations, zero isolation. Replacement microbench: no-op FFI region looped 100× isolates pool/barrier cost directly.

**8. Ledger — accepted, structurally wrong not just optimistic.** Phases 1 and 3 spend the same per-leaf floor twice; 5b was air; 7/10 inputs didn't exist. v2 **budgets terms and treats phases as spend paths** — phases never sum again.

**9. Accepted. Phase 9 is written below**, with a derived ceiling instead of a ban.

**10. Accepted — fixed by moving the measurement to the front.** The κ session moves into Phase 0 (one afternoon, existing binaries, zero porting). It writes Phase 7's real number and prices Phase 9 *before* a line of porting.

**11. Accepted.** Absolute MAD is weather, not a gate. New statistic: interleaved pairs, d_i = t_Titan − t_pc within pair; report median(d) and MAD(d); claim requires |median(d)| > 2×MAD(d). Raw MAD > 0.25s aborts the *session* (too hot, reschedule), doesn't fail the rig. Pairing cancels common-mode thermal drift — ~20–24 pairs suffice.

## Law 1′ (replaces Law 1)

Core classes c with counts N_c (A78: 2, A55: 6); per kernel k, pinned measured rate s_k^c; divisible work W_k:

**wall = min over assignments of max_k [ W_k / Σ_c n_k^c·s_k^c ]**, Σ_k n_k^c = N_c.

21 combinations for two kernels on this chip — enumerate at runtime. β=5.5, δ=9.0 as fitted are *mix-contaminated* thread-seconds; the κ session re-fits them per class.

**Matching corollary (this is the load-bearing result):** if s_k^A78/s_k^A55 is equal across kernels, assignment is irrelevant — wall = work/capacity, and your 1.79s ≈ the 1.81s uniform bound says you're already there. **All scheduling upside on this chip is differential-κ.** The κ table is therefore the single most valuable measurement in the plan: it prices Phase 7, prices Phase 9, and bounds both before either is attempted. If κ_B ≈ κ_D and within-term κ is flat, Phase 9.5 is a no-op — and you know that in week one, not month three.

## Phase 9 — B/D native (the missing phase, scoped)

**9.0 Oracle hardening (½ day).** We build the primecount we FFI into, so instrument *our* build: env-gated per-b and per-segment partial dumps, deterministic combine order. Oracle upgrades from "term sums match" to "partials match" — catches port bugs and lucky races that preserve totals. Same-process FFI‖native shadow runs.

**9.1 B port, mechanical (2–4 days).** Zero restructuring. Gate: term bit-identical, per-b partials identical, wall within ±2% of FFI-B. The tie *is* the deliverable — it certifies the port before any surgery.

**9.2 D port, sliced (4–7 days).** FactorTableD first (port the 2B mpf-fused encoding faithfully — no "improvements" during port), then segment loop, then balancer. D is the risk concentration; his atomic balancer is where port bugs hide. Law 2 + partial diffs is the net.

**9.3 Pole-side surgery.** Refit under Law 1′ with the κ table. Only the pole term gets deletions: u32 quotient buckets in B leaves (range-asserted per disposition 5), invariant hoisting, segPi-style tables, the isqrt-walk *where a per-visit sqrt actually exists* (verify first — disposition 6 applies to me too). PhiCache only if B is pole-side and instrumented φ hit-rate justifies the bytes.

**9.4 Fusion substrate.** One region, typed task families (AC-seg, C1-b, B-batch, D-batch), one pool, all tables once. Direct wall gain ≈ 0 — conceded. Value: assignment granularity drops from whole-region to task-family, which is what 9.5 needs.

**9.5 Personality on B/D.** Assignment from the Law 1′ enumeration; size classes 32KB/64KB by L1D; stretch version splits B *itself* by sub-work-type (recursive descents → A78, streaming scans → A55). Gated on differential κ. If flat, this is a finding, not a failure.

**9.6 Stretch.** PhiCache above his x^1/2.3 cap under measured bandwidth headroom; NEON u32 magic in B leaves; cost-weighted dispatch.

**Pre-9 freebie, run in Phase 0:** we control the primecount build — add a pinning shim and enumerate ~10 pinned FFI configs (B's 3 threads to {2×A78+1×A55}, D to A55s, and neighbors) against Law 1′. If differential κ exists, −0.05–0.15s lands before any port.

**Honest expected:** 9.1–9.2: 0 (certification). 9.3: −0.04–0.10. 9.4: 0. 9.5: −0.10–0.30 *iff* κ spread ≥ ~1.3×. 9.6: −0–0.08.

## Ledger v2 (budgets, not sums)

| term | now | p50 | ceiling | opponent |
|---|---|---|---|---|
| AC | 1.40 | 0.95–1.10 | 0.85 | 0.96 |
| B+D | 1.79 | 1.79 → 1.55–1.70 (P9, κ-gated) | 1.45 | 1.82 |
| σ/φ0 + overheads | ~0.16 | 0.07 | 0.05 | ~0.16 |
| **FULL** | **3.35** | **2.65–2.85** | **~2.45** | 2.96 cold / ~3.05 fair |

p50 win 8–12%, tail ~18–20% under fair protocol — landing where your critique put it, not where v1 claimed.

## The ceiling (derived, not banned)

2× at 1e16 means FULL ≤ 1.5s. B+D is at work parity with his code; the capacity bound on 2×A78+6×A55 — even granting κ = 2 on the pole term — puts the B+D floor at ~1.4–1.45s. AC floor is ~0.85 (his is 0.96 and he's not stupid). Sum ≥ ~2.3 ⇒ **max win ~1.25–1.35×.** Beating that requires deleting ~40% of B/D *work*; his B/D already carry clustered leaves, PhiCache, FactorTableD — no known deletion of that size exists short of new algorithmic structure, which is a research project, not a phase. So Phase 9 is built to reach the ceiling, and the strongest true sentence available is: *every term at-or-below primecount, p50-certified, on silicon his binary cannot see* — across 10¹²–10¹⁸, with 10¹⁹ RSS-gated as before.

## Session 1 (kickoff, unchanged in kind, re-ordered in content)

Phase 0 rig + **κ table** (pinned per-class rates through FFI-B, FFI-D, native AC kernels) + pinned-enumeration freebie + Phase 1 templating. Bring back the κ table, the pinned-enumeration result, and per-term paired medians. Decision that day: κ flat → cut 9.5 and Phase 7's headline from the plan; κ spread ≥ 1.3× → the endgame is real and 9 goes on the calendar.

The skeleton survives; every number in it now has to be measured, proven, or deleted. The critique's last line was right — v2 is the surgery, and the next artifact that matters isn't this doc, it's the κ table.
