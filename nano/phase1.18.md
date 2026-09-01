# It Didn't Get Fucked — It Got Audited by Repetition

Read the run correctly before the frustration sets in: **the engine is fully intact.** 13/13 unit tests, 20/20 identity points bit-exact, all five Mertens anchors exact. What died last night was not code — it was **Phase 15's ledger numbers**, and the fresh session is the first time this project has ever re-run a measurement and *caught its own previous number failing to reproduce*. After eight phases of single-run claims, the instrument finally bit. This is the system working. Now the autopsy, because the fresh numbers contain three findings, and the second one changes the phase's structure.

## F1 — The 39% phantom: Phase 15's 10¹³/10¹⁴ timings were unreproducible

| Scale | Phase 15 report | Fresh (unthrottled) | Delta |
|---|---|---|---|
| 10¹² | 0.3056 s / 118.16 cy | 0.2876 s / 112.59 cy | **fresh is 6% faster** |
| 10¹³ | 1.0322 s / 93.77 cy | 1.4292 s / 130.30 cy | **fresh is 39% slower** |
| 10¹⁴ | 5.5414 s / 118.19 cy | 6.5714 s / 138.62 cy | **fresh is 17% slower** |

Thermal throttling cannot produce this shape — a throttled session is slower at *every* scale, and your 10¹² row got faster. The discriminating evidence is the α-sweep: its α=1.0 rows (1.391 s at 10¹³, 6.5275 s at 10¹⁴) **match the fresh ledger almost exactly** — the current code is internally consistent across two independent binaries. The old 0.9533 s is an orphan with no corroborating measurement anywhere in the project. And look at the two curves: the old sawtooth (118 → 94 → 118) was incoherent with any physical story; the fresh monotone (112.59 → 130.30 → 138.62) is a fingerprint, and F2 reads it.

**Law (run-protocol amendment, permanent):** every perf number in every ledger is **min-of-5 in a cool session, with the spread (max/min) printed beside it**; spread > 1.25× quarantines the session as unstable. One protocol line would have caught the 39% phantom the day it was minted. The config-digest law becomes load-bearing in the same commit — if the code state differs between the old and fresh runs, the digest *says so*; if it doesn't, the old number was luck. Either way, the question never survives on vibes again.

## F2 — The monotone curve is a cache fingerprint, and it projects *badly* at marathon scale — this is the finding that matters

Compute the random-access working set per scale: the π-table at 10¹² spans x^½ = 10⁶ → **~100 KB (L2-resident)**; at 10¹³ → **~316 KB (borderline)**; at 10¹⁴ → **~1 MB (L3-class latency)**. The walker's cost is three random lookups per cell into these structures — so cy/cell *must* grow as the tables cross cache thresholds. 112 → 130 → 139 is exactly that crossing. The attribution I predicted for K0 (three unamortized random L2 bounces ≈ 90–120 cy) is now confirmed by the growth curve itself, before the attribution counter even runs.

Now extrapolate honestly, because this is the strategic escalation: at 10¹⁷ the π-table spans 3.16×10⁸ → **~30 MB**; at 10¹⁸, **~100 MB** — pure DRAM-latency territory. Random lookups into that cost 60–100+ cycles each: **cy/cell degrades to ~200–300 at marathon scale.** The current kernel doesn't just lose the constants war at 10¹⁴ — it *structurally worsens* with scale.

The counter-move is K2, and this finding promotes it: **π-streaming is not an optimization rung — it is a marathon prerequisite.** Sequential prefetch is scale-robust (a stream from DRAM costs the same per element whether the table is 1 MB or 100 MB); random access is not. The K2 kernel (per-j monotone-v block-walking with carried state) is the difference between a walker whose cost is flat in table size and one whose cost grows with the domain forever. Same for the Mertens stream in K1. This is the entire reason the monotone-v design was specified — the fresh run just proved it's load-bearing rather than merely profitable.

## F3 — The α-dial inverted my prediction, and the inversion is the finding

I predicted α ∈ [1.5, 2.5] recovers a meaningful fraction. **Measured: α = 1.0 is optimal at both scales, and the dial is monotone the wrong way** — walker time rises with α (1.29 → 2.07 s at 10¹³), sweep time falls (0.10 → 0.04 s), and since the walker is 94% of total, the sweep's 4–5× shrink buys ~6% against the walker's +60%. My prediction was wrong at the current cost structure, and the reason is structural, worth stating precisely: **primecount's α ≈ 2–3 tuning assumes a cheap special-leaf engine.** Their sweep is expensive, their leaves are cheap, so they dial up to shrink the sweep. Ours is inverted — walker expensive, sweep cheap — so their dial points the wrong way for us, and *borrowing Gourdon's tuning constants was never legal.* Recorded: **α := 1.0, current default, per scale, into the profile.**

And the re-dial trigger, written into the profile now: when the K-rungs drop the walker below ~25% of total, the dial re-runs — at that point α ≈ 2–3 plausibly wins by trading a cheap walker for a smaller sweep (e.g., post-K2 at 10¹⁴: walker ~0.3 s + sweep 0.14 s at α=3 ≈ 0.45 s vs 0.78 s at α=1 — the dial flips). The α-sweep's job was never to save today; it was to map the tradeoff — and it will be re-read after the terrain changes.

## F4 — The banner, briefly, because it's now load-bearing

`v1_suite` printed **"COMPLETE AND FULLY CERTIFIED"** over a table that *fails four of its own committed thresholds* — collapse 3.01× vs ≥ 15×, π(10¹⁴) 6.57 s vs 2.5 s floor. The suite computes numbers and compares nothing; the lying-banner pattern has now infected its ninth host binary. The fix is one commit and it rides with K0: **thresholds in, verdicts emitted, exit = FAIL count** — the calculator law, finally in the newest code instead of only in the specs. Same message, shorter: the 95.2% scoreboard certifies a Phase 16 that has run nothing, the marathons are owed for the ninth phase, and the perf ledger still doesn't print π(x) *values* at ledger scales — an engine whose ledger shows times but not counts. All standing debts; none new.

---

## What changed, what didn't

The war plan is unchanged: **K-series, one commit per rung, ±3% keep-or-revert, oracle-quick between.** The starting line moved from 118 to 139 at the worst scale, K2 got promoted from rung to prerequisite, α got a default, and the marathon budgets get restated from measured numbers only:

| Milestone | As-measured (139→250 cy at scale) | Post-K1/K2 (streaming, ~30–40 cy) |
|---|---|---|
| π(10¹⁴) | 6.57 s (23.9× behind) | **~1.7–2.1 s** (K3: ~1.5–1.9 s) |
| **Marathon I (10¹⁷)** | walker ~12–15 min + sweep 33 s → **≤ 30 min sustained — runs TONIGHT, as-is** | ~1.5 min |
| **Marathon II (10¹⁸)** | ~70 min cool / ~2 h sustained (fallback floor) | **~12 min cool / ~20–25 min sustained — the committed target** |
| 10¹² crossover vs pc | — | needs cy ≤ ~20: **0.06 s vs their 0.0829 s = the win column** |

The gate stays **cy/cell ≤ 35 at 10¹⁴ (floor 45)** — the committed crossover arithmetic barely depends on the baseline, because it was always about the target, not the starting line.

**Run order:** Marathon I tonight on the substrate as-measured — ninth phase owed, honest budget is comfortably runnable, nothing precedes it, and K0 + the verdict-emission fix + the run-protocol ride the idle cores during the run. Then K1 (M-chaining — the free 25–30 cycles the run-boundary identity hands us), then **K2, the prerequisite**, then Marathon II, then the α re-dial, then re-race v3.

Paste back: the M-I cert-record raw, K0's attribution table (≥ 90% of the *139* attributed — the target moved with the baseline), and each rung's cy/cell line as it lands. The fresh session didn't set the project back — it set the project *down on solid ground*, for the first time with a number that repeats. The war starts from 139, and the weapon it needs first is the one the growth curve just demanded by name.
