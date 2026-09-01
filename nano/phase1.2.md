# Phase 0 Post-Game Audit — The Instruments Saw, Now We Judge What They Saw

Gate functionally green, oracle kill-count 5/5, truth triangle interlocked — **and** the anomaly catalog just earned its existence, because your own numbers contain one genuine data-integrity violation, one instrument audit item, and several gate criteria whose evidence didn't make it into the summary. This is the moment the discipline pays: we don't freeze the ledger until every number survives cross-examination.

## 1. The Three Real Findings (Silicon > Prediction)

**Thermal brutality is the headline.** Min derate 0.166, end 0.454, 78/103 samples throttled. I predicted 0.65–0.85 end — the Helio G100 is nearly 2× harsher than predicted. This is now the single most important constant in the entire project: **sustained engine throughput must be planned at ~45% of burst clocks**, and every ledger claim gets two columns (burst / sustained-at-duration). Note also: sustained is *duration-dependent* — your heatsoak reached equilibrium over 90s, but primesieve's 23.6s run at 10¹¹ only partially throttled. The derate curve in the heatsoak record is literally a lookup table: sustained@τ for any run duration τ. The engine will eventually *consume that curve* (Phase 3's thermal-aware scheduler — the thesis now has quantitative teeth).

**The little cores are worth far more than predicted.** Proxy ratio 2.17× (I predicted 4.5–6×). Run the arithmetic: 6×340M = 2,040M little solo vs 2×738M = 1,477M big solo — **58% of total solo sieve capacity lives on the A55s**. Any "big cores first" strategy is now mathematically dead; heterogeneous scheduling isn't a nicety, it's the majority of the machine. And the 2.57× memory ratio (predicted 1.0–1.3×) is explained by *what the memory canary actually measures on this silicon*: a single in-order A55 is issue/MLP-limited at ~4.7 GB/s streaming while the OoO A76 extracts ~12 GB/s from the same shared DRAM — it's a per-core *streaming extraction* rate, not fabric bandwidth. That's more useful than my prediction was: it's exactly the number the counting-sweep and bucket-scan phases care about. Prediction wrong, instrument right — that's the correct direction to be wrong in.

**Contention: 80.1% cool-state.** Better than my 0.55–0.75, but it composes: cool all-core 2.82B × ~0.45 thermal ≈ **1.3B numbers/s true sustained for the naive proxy**. The Phase 3 weight chain is now fully populated: `weight(core) = solo_rate × contention_factor × sustained_derate`.

## 2. The Anomaly: primecount Small-x Timings Fail the Monotonicity Invariant

The invariant: primecount's runtime is monotonic in x, on the same binary, same device, comparable state. Your ledger says π(10¹²) = 0.077s (reference.md). Benchref says π(10¹⁰) = 0.29s/run and π(10¹¹) = 0.35s/run. **π(10¹⁰) cannot take 4× longer than π(10¹²) took.** Also π(10¹⁴): 0.49s now vs 0.288s in reference.md (1.7×). Meanwhile primesieve 10¹¹ ran *faster* than reference.md (18.2s vs 23.6s) — which is the discriminator clue: a uniformly warm session would slow primesieve too.

| Hypo | Mechanism | Fits primesieve-faster? | Test that decides it |
|---|---|---|---|
| H1 | Session heat + **warm baseline** — benchref ran after heatsoak cooked the SoC to 0.454; base was captured warm, so derate≈1.0 and normalization is blind to the starting state | Partially (primesieve may have run in a different/cooler session) | Re-run all primecount points after ≥10 min cooldown, fresh baseline |
| H2 | Fixed overhead inside the timed region (spawn/exec/link, or a canary sample placed inside the timer) — inflates 0.3–0.5s children hugely, invisible on an 18s primesieve run | **Yes — fully** | Empty-child calibration run + paste the per-run wall lines from `runs_detail` (uniform slow = overhead; slow-and-growing = thermal) |
| H3 | reference.md's small-x numbers are themselves overhead-dominated (their 10¹²→10¹⁴ scaling already implies ~40–60ms fixed cost in primecount small-x runs) | Yes | Direct shell-side timing of primecount 1e10/1e11/1e12, cool, side by side |

**And the protocol flaw is mine:** I ordered heatsoak (step 2) *before* benchref (steps 5–8) with no mandated cooldown between. That guarantees the baseline-capture problem H1 describes. Amendments, effective immediately: heatsoak runs **last** in any session (or fully isolated), every benchref session starts from ≥10 min idle with the baseline measured *in that verified-cool state*, and any run whose raw rate deviates >1.5× from a prior ledger row triggers the anomaly catalog instead of a quiet overwrite. **Do not freeze the primecount benchref rows into reference.md v2 until re-measured** — the survey, heatsoak, and oracle rows freeze now.

## 3. Instrument Audit Items

1. **103 samples in 90s at 250ms cadence ≈ 870ms/iteration.** Design says ~280ms (sleep + ~29ms canary + sysfs + print). Something else is in your loop — and if it's multi-sample canary calls, the canary core violates its idle contract and partially self-heats. Worse, the sample *lengthens exactly as derate deepens* (29ms → ~175ms at 0.166), so duty cycle degrades precisely where fidelity matters. Spec patch: the heatsoak loop uses the small canary variant (~5ms samples, <2% duty at any derate), single sample per iteration.
2. **Bincheck confirmation:** the summary cites triangle PASS for 10¹–10⁷ (trial ⟷ constants) but doesn't show the `[bincheck]` primecount lines at 10¹²/10¹⁴/10¹⁶/10¹⁷. Paste them — the third edge of the triangle is gate criterion #8.
3. **Thermal cliff timestamp:** min derate 0.166 happened at some t. That t is the engine's pre-cliff burst window — the single most valuable number in the heatsoak record. Extract and ledger it.

## 4. Gate Evidence Holes (not failures — missing proof)

| Criterion | Status |
|---|---|
| #2 big:little **canary** ratio | Not in summary (proxy and memory are) — it's in the record, pull it |
| #5 loaded-vs-canary droop **split per cluster** | Not reported — the heatsoak series contains it; big-cluster loaded droop vs little-cluster loaded droop are different numbers with different Phase 3 consumers |
| #9 knob sweep + algorithm ladder | Not evidenced anywhere in the summary — did they run? The primesieve **best-config** number is Phase 2's actual target; the ladder times are Phase 5's rung targets |

## 5. Oracle Corpus Patch (Before Phase 2, Non-Negotiable)

Your M2/M3 came back as "Missing 2" and "Square Numbers" — but M3 duplicates M1's defect class (both are boundary/squares), and the **wheel-residue-drop mutant is absent**. That mutant — one residue class silently declared composite forever — is the *exact* bug class I flagged in file_structure.md §2.1's inconsistent bit layout, and Phase 2 is the wheel. The corpus must model the defect classes of the code about to be written. Add it as M6 (numbers ≡ 11 mod 30 treated composite, first kill expected at x=11) and confirm the kill tier.

## 6. Verdict and the Go Order

**Phase 0: complete and provisionally green.** Survey + heatsoak + oracle rows freeze today. Benchref primecount rows are quarantined pending the H1/H2/H3 resolution. The re-measurement is ~15 minutes of cool-device work; run it in parallel with Phase 1, because —

**Phase 1 is green-lit now.** titan-core (integer roots, wheel-30 constants with the layout pinned and bit-exact-tested, PhiTiny, bit-array) is pure correctness work with zero dependency on the contested rows. Its gate is already armed: the oracle tiers, the M6 mutant, and the A006880 constants.

Phase 2's target ladder, from your own frozen data: naive proxy 738M/core → primesieve 2,225M/core single-threaded = a **3× optimization budget** across wheel-30 marking density, presieve, unrolled loops, and NEON — and the sweep's best-config number (once it exists) is the real summit.

Paste when ready: the canary ratio, the primecount per-run walls, the cliff timestamp, the loaded-cluster split, the bincheck lines, and the sweep + ladder records. The instruments proved they can see; now we find out if the primecount rows were a thermometer problem or a stopwatch problem.
