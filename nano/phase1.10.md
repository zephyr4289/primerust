# The Silicon Transplant — Then Phase 8: The Second Device

Stop and absorb what just happened, because it's the largest methodological event since Phase 0: **every performance number in this project was calibrated on a machine you no longer hold.** The canary baselines, the 14.5 s thermal cliff, the 0.454 sustained derate, the 11.44 GB/s DRAM knee, the 64 KiB L1D summit, the E1 weight vector, the 9.15 B/s MT summit, every primesieve/primecount reference floor, and every gate threshold derived from any of these — all of them are now *historical artifacts of the Helio G100*, and your Phase 7 gate just ran on top of them on Snapdragon 4 Gen 2 silicon. The report treats the device as a footnote. It is not a footnote. It is the entire calibration layer of the ledger.

Same 2+6 topology, yes — but topology is the *coarsest* description of a machine. Cortex-A78 ≠ Cortex-A76 (deeper out-of-order window, better memory-level parallelism — the exact properties the bucket engine and state streams feed on). A78's L1D is implementation-configurable at **32/48/64 KiB** — which means, hard stop: **the 64 KiB segment geometry currently in the engine may be actively wrong on this phone**, spilling every segment out of L1 on the big cluster. That alone could be a double-digit fraction of the 22.65 s at 10¹⁴. The memory controller may be LPDDR5 or LPDDR4X depending on the board — the 11 GB/s wall may have moved or not. The chassis and thermal solution are different — the thermal model that predicts sustained rates through the telemetry chain is gone. And the *engine didn't notice any of this* — it compiled, ran, and produced bit-exact π(x) to 10¹⁴ on new silicon with zero code changes.

That last sentence is the architecture's finest validation: the wheel, the identities, the oracle tetrahedron, the explicit-stack Φ, the zero-alloc discipline, the self-balancing pool — all silicon-agnostic *by construction*, and a live transplant just proved it. Correctness transferred 100%. Performance transferred 0%. The project now needs a phase whose entire job is rebuilding the calibration layer for the second device — and the beautiful part is that this costs *minutes*, not weeks, because Phase 0's instruments were designed to be re-runnable. That was never ceremony. This is why they exist.

## Phase 7 Audit Under That Light

**Genuinely green:** the Meissel identity is certified (anchors through 10⁷, p³±1 matrix, 20-point Lehmer differential, 10¹²–10¹⁴ exact against A006880 — the values in [5/6] all match). D-lock shipped as a real derivation document. C10 ran *before* the code and empirically confirmed the architecture theorem with a margin worth framing: **max leaf y = 3,156,176 against √10¹³ = 3,162,277 — the bound held with 0.19% slack.** The theorem is exact, the census is the instrument that proved it live, and the retro-audit now *visibly* carries Phase 4's 8 debts instead of hiding them — the scoreboard itself got more honest. Progress, acknowledged.

**The failures, one of which is the third strike on the same law:**

1. **The gate printed ALL GREEN with π(10¹⁴) = 22.651 s. The Phase 7 spec said ≤ 3.5 s cool, floor 5 s.** Even granting every device-switch excuse in advance, 22.65 > 5.0 — this is FAIL printed as PASS, in the phase that *created* the gate-contract law. The gates are still self-defining their criteria internally instead of executing the contract. The fix must now be mechanical and numeric — see Law 1 below.
2. **L7.0 (contract re-diff) demonstrably did not run.** The scoreboard still shows 5/4/6/9 criteria for Phases 0–3 against original spec tables of 10/10/10/12 — **18 criteria silently vanished from accounting.** Your 89.2% completion rate is computed against the consolidated denominator: 66/74. Against the original 92 criteria, the true pass rate is **71.7%**. The consolidation must become visible commits or be restored, and the completion rate must be recomputed against the honest denominator.
3. **Five of six mutants owed.** M-s2-sign died; M-mu-sign, M-leaf-boundary, M-j-offset, M-sweep-top, M-mertens-partial, M-alpha-domain are unreported. Marathons I and II — the phase's crown deliverables, criteria 9–10 — not run. Differentials in [10¹⁵, 10¹⁶] — criterion 4 — not run. C11/C12/C13, the α-sweep, the Gourdon rungs — none evidenced. No term-level ledger at 10¹⁴ (Phase 5 had one; this gate shows totals only — I cannot decompose the 22.65 s into table/leaves/sweep without it).
4. **Naming honesty:** what shipped is **Meissel** (a = π(x^⅓), P₃ ≡ 0) — the L7.2 rung. The special-leaf machinery and Gourdon refinements (L7.3+, L7.6) are the actual LMO/Gourdon contributions and are not in evidence. The ledger reclassifies: "Meissel-class" until the rungs land.
5. **The cross-device contamination, stated precisely:** Meissel@4Gen2 (22.65 s) cannot be compared against Lehmer@G100 — different silicon invalidates the comparison *and* the class-gain claim. The 20× algorithmic gain exists only in theory until both engines run on the same device.

And the three-hypothesis diagnosis of 22.65 s, which Phase 8 resolves by measurement: **(a)** stale 64 KiB geometry on possibly-32-KiB A78 L1D; **(b)** un-run optimization rungs (no α-tuning, no special-leaf work); **(c)** an unknown thermal envelope on new silicon in a new chassis. My money is on all three contributing, in unknown proportions. That is exactly what instruments are for.

---

# Phase 8 Engineering Specification — The Second Silicon: Re-Instrumentation, Retuning, and the Marathons

## PART 1 — MANDATE AND LAWS

**Scope:** full re-instrumentation of the 4 Gen 2 (P-series), reference re-baselining, the same-device engine matrix, Phase 7 debt closure, the retuning ladder, Marathons I & II. **Deferred to Phase 9:** the product phase (CLI, streaming, cross-arch packaging, finale ledger) — it inherits a *device-profile-aware* engine instead of a G100-tuned one.

1. **The numeric-contract law (Law 0's teeth, final form).** Contract files carry *numeric thresholds*, not prose. Gate binaries are executors: load contract, measure, compare, emit PASS/FAIL/OWED, exit = non-PASS count. A gate can no more "print PASS at 22.65 s against a 5 s floor" than a calculator can print 4 at 2+3. Contract provenance is enforced by hashing the contract against the phase spec's table — the 18 vanished criteria become impossible to lose silently.
2. **The device-profile law.** Every calibrated constant lives in a device profile: {topology, E1 weights, contention factors, thermal curve, DRAM knee, geometry defaults, cluster tags}. The pool's front-load, the segment geometry, and the thermal model read from it. No constant is baked into code. The G100 profile is frozen as history; the 4 Gen 2 profile is this phase's deliverable.
3. **The ledger-fork law.** Every record gains a device field. reference.md forks: Section A — Helio G100 (frozen, historical); Section B — SM4450 (live). **Cross-device comparisons are forbidden in all tables** — a number is only ever compared to a number from the same machine, recorded in the same session, per the Phase 0 comparability law, now extended to its final dimension.
4. **The baseline-key law (spec bug fix).** `baselines.json` was keyed by rustc version only. It is now keyed by **(device, rustc)** — the comparability chain was incomplete at the device dimension, and the transplant just proved it.
5. **Purity, pool, one-pass, field-domain, differential-extension laws** — unchanged, and already proven portable by the transplant.

## PART 2 — RE-INSTRUMENTATION (The P-Series; ~1 Hour of Device Time)

The entire Phase 0 instrument stack re-runs — canary, survey, heatsoak, knee — because it was built device-agnostic. Order matters:

| # | Instrument | Question on 4 Gen 2 | Prior band (honest unknowns) |
|---|---|---|---|
| P0a | Canary spot-check, all 8 cores | Core-speed delta A78-vs-A76, A55 clock | big:little ~3–4×; absolute canary ±? |
| P0b | Full survey + E1' with the *real engine* | The weight vector; per-core sieve rates; contention | A78 ~2.2–3.0 B/s; A55 ~0.7–0.8 B/s |
| P0c | **Geometry re-sweep (the urgent one)** | L1D implementation truth: 32 vs 48 vs 64 KiB summit per cluster | 32 KiB or 64 KiB — the sweep *is* the answer |
| P0d | F1' DRAM knee under the real access mix | LPDDR4X vs LPDDR5 wall | 11–25 GB/s |
| P0e | Heatsoak' 90 s + 180 s | New thermal curve, cliff time, sustained derate | derate 0.4–0.8, cliff 10–30 s — new chassis, new node, no priors worth trusting |
| P0f | Hygiene/environment | Termux-API availability, pinning permissions, RAM inventory (Marathon II needs ~400 MB working set), new rustc key | — |

The 30-second canary delta (P0a) runs *first* — it prices the transplant in one number before anything else moves.

## PART 3 — REFERENCE RE-BASELINING

primesieve + primecount re-benchmarked on the 4 Gen 2 under benchref, cool-session protocol, canary-sandwiched, both columns. The G100's 9.15 B/s summit and 2.756 s Gourdon-at-10¹⁶ are **retired to Section A**. Every future "we beat X" claim compares against Section B numbers, same session, pinned N, best-config swept (`--sieve-size` sweep included — their optimum also moves on this silicon). This is non-negotiable: the opponent must be measured on the same battlefield.

## PART 4 — THE SAME-DEVICE ENGINE MATRIX (The Diagnosis Instrument)

All three engines on the 4 Gen 2, one session, cool, telemetry-recorded: **titan-sieve, Lehmer (Phase 6), Meissel (Phase 7)** at 10¹²/10¹³/10¹⁴, with term-level ledgers restored (Phase 5's discipline — the 22.65 s gets decomposed into table-build / leaf-enumeration / sweep / assembly). Deliverables:

- The honest class-gain table: Meissel vs Lehmer *on identical silicon* — the number the Phase 7 report wanted but could not produce.
- The three-hypothesis decomposition of the 22.65 s: geometry (re-sweep A/B), rungs (special-leaf present/absent), thermal (telemetry curve).
- A re-derived cost model for the Meissel class on this device — sweep rate from E1'/F1', leaf-machinery constant from the measured term ledger, table-build from the certified sieve rate. **Every subsequent target in this phase is derived from this model, not guessed.**

## PART 5 — PHASE 7 DEBT CLOSURE (On the New Device, Under the New Law)

- **Mutants ×5** (M-mu-sign, M-leaf-boundary, M-j-offset, M-sweep-top, M-mertens-partial, M-alpha-domain) — killed with tiers recorded; the corpus is device-independent, so nothing is re-derived, only executed.
- **Randomized differentials vs primecount in [10¹⁵, 10¹⁶]** — the differential-extension law's range, free truth at ~0.1–3 s per point.
- **Cross-engine extension through 10¹¹** (Phase 7's topped out at 5×10¹⁰).
- **C11 α-sweep** at 10¹³/10¹⁴ (sweep-span vs leaf-machinery tradeoff on this device), **C12 one-pass overhead** (<10% mandate), **C13 primecount ladder at 10¹⁵/10¹⁶** — 4 Gen 2 reference rungs for Phase 9's honest gap table.
- **Contract re-diff, executed this time**: the 18 consolidated criteria restored or reconciled by visible commit; completion rate recomputed against 92; that number goes in the ledger even if it's ugly. *Especially* if it's ugly.

## PART 6 — THE RETUNING LADDER (One Rung Per Commit, ±3% Keep-or-Revert)

**T0** device-profile wiring (pool weights, geometry defaults, thermal model read from profile; G100 frozen) → **T1** geometry fix from P0c — *if* the summit moved to 32 KiB, this alone may claw back a large fraction of the 22.65 s → **T2** Meissel micro-retuning on new silicon (leaf enumeration layout, sweep unit sizes — the R2/SoA discipline, fourth consumer) → **T3** **the special-leaf machinery** — the actual LMO contribution, the un-run mass from Phase 7, now built on the re-derived cost model → **T4** α-tuning from C11 → **T5** Gourdon refinements (z-split, Σ-sharing) — the rungs that justify the "Gourdon-class" name, and the ledger reclassification when they land → **T6** vector kernels where the A78's window rewards them (NEON leaf counters, tally — re-certified via the scalar differential as always).

## PART 7 — MARATHONS I & II ON THE 4 GEN 2

The 10¹⁷/10¹⁸ runs, with targets **derived from Part 4's cost model and committed to the record before execution** (that is the honest form of a target on newly-characterized silicon — predicted-cool × 1.5 is the gate bound; beating it is a badge, missing it is an audit). Envelope checks close cleanly: sweep at 10¹⁸ spans x^(2/3) = 10¹² numbers ≈ 508,626 segments ≪ the u32 rem_segs ceiling of 4.29×10⁹ (the rem_segs fix, validated at its new domain); sieving primes ≤ 10⁶ within prime:24 and the p ≤ 10⁸ assert; π-table ≈ 255 MB by the 4 Gen 2's own density measurement (P0f confirms RAM headroom). Checkpoint protocol unchanged; **crash gauntlet ≥ 5 kills mid-10¹⁷**, bit-exact resumes; the C10 leaf-bound check runs as a *runtime assert* during marathons (the 0.19% margin at 10¹³ earned that promotion). Differential spot-checks vs primecount in [10¹⁶, 10¹⁸] after each marathon — the truth is cheap at every scale, and the rem_segs lesson says use it at the top of the domain.

**Marathon II is the badge: π(10¹⁸) = 24,739,954,287,740,860, exact, phone-native, on the *second* phone to carry this engine — a silicon transplant between the certification of the algorithm and the computation of the record.**

## PART 8 — THE GATE (numeric contracts; exit = non-PASS count; every record device-tagged)

| # | Criterion |
|---|---|
| 1 | Contract re-diff executed: 18 consolidated criteria restored/committed; completion rate recomputed against 92 and recorded |
| 2 | P0a–P0f records exist; device profile (SM4450) committed; G100 profile frozen; baselines keyed (device × rustc) |
| 3 | Geometry summit re-swept per cluster; engine defaults match it; ledger records whether 64 KiB was right or wrong on this silicon |
| 4 | F1' knee + heatsoak' curve recorded; thermal model in profile; sustained predictions flow through telemetry chain (Phase 3 law, re-validated) |
| 5 | References re-baselined (primesieve/primecount, both columns, best-config); Section B of reference.md live; zero cross-device comparisons anywhere |
| 6 | Same-device engine matrix: term ledgers at 10¹²–10¹⁴; Meissel-vs-Lehmer class gain established on identical silicon; 22.65 s decomposed into (a)/(b)/(c) with numbers |
| 7 | Phase 7 debts: 5/5 remaining mutants killed; differentials [10¹⁵, 10¹⁶] ≥ 5 points bit-exact; cross-engine through 10¹¹; C11/C12/C13 recorded |
| 8 | Retuning ladder: T1–T3 landed with keep/revert verdicts; π(10¹⁴) target **derived from the re-derived cost model, committed in advance**, gate = within 1.5× |
| 9 | Marathon I: π(10¹⁷) = 2,623,557,157,654,233, cert-record, ≥ 5 kill-resumes bit-exact, leaf-bound runtime assert green |
| 10 | Marathon II: π(10¹⁸) = 24,739,954,287,740,860, cert-record, ≤ derived-target × 1.5, differential spot-checks above 10¹⁶, checkpointed, kill-resumed |
| 11 | Zero-alloc tripwire with 8 workers on new silicon; sync inventory unchanged; field-domain audit re-confirmed at the 10¹⁸ envelope |
| 12 | reference.md forked A/B; gate record persisted; Phase 9 preview committed (device-profile-aware product) |

## PART 9 — AMENDMENTS AND DECISION MAP

**Amendments (v2.6):** the device-profile law and ledger fork; the numeric-contract law with contract hashing; baselines keyed (device × rustc); Phase 7 engine reclassified Meissel-class pending T5; marathon targets on new silicon are *derived-then-committed*, never asserted; the C10 leaf-bound check promoted to marathon runtime assert.

**Decision map:** the device profile → every consumer of a calibrated constant (pool, geometry, thermal, MT scheduling) and the Phase 9 auto-tuner; the same-device matrix → the honest algorithm-class ledger that Phase 9's finale table publishes; C11's α curve → per-scale defaults on this silicon; F1' knee → whether the 6-byte entry rung and further byte-economy matters here (if the wall lifted to LPDDR5, priorities shift toward cycles); the marathons → the capstone records; and one open question that changes Phase 9's shape: **do you still have the G100?** If yes, it becomes a second CI node — every gate runs on both profiles, and the project's final claim gets stronger: *an engine, not an engine-for-one-phone.* If no, Section A stands as a closed historical archive, and the transplant story becomes part of the record — the engine that moved between machines mid-flight and stayed exact.

---

Run order: **P0a canary first — one number, thirty seconds, prices the transplant.** Then the survey, then the geometry sweep (T1 is potentially the cheapest large win available on this phone), then the matrix, then the debts, then the marathons. Paste back the canary delta, the geometry summit, and the same-device Lehmer-vs-Meissel table — those three numbers decide whether 22.65 s was the silicon, the geometry, or the missing rungs, and everything after them is execution.
