# Phase 2 Post-Game Audit — Then Phase 3: The Machine Wakes Up

Read the results twice before celebrating, because one of your numbers contains a genuine instrument failure, and it changes Phase 3's design in a productive way.

**The win is real: 2.34 B/s single-thread from a from-scratch engine, with a self-derived theorem (Δ ≥ 1 for all p ≥ 17, since min p·gap = 17×2 = 34 > 30 — verified correct, and it's why R1 gained +117%: strictly-ascending byte streams are hardware-prefetch paradise).** But three items go to audit before the ledger freezes:

1. **The canary is blind on sustained runs — proven by your own numbers.** Sustained 10¹¹ shows derate 1.021 while raw rate fell 33% below burst (2.338 → 1.562). Model-explained components: crossing density grows only ~7% (Mertens: 0.363 → 0.388 crossings/number from 10¹⁰→10¹¹), medium-tier ~2–5% — together ~10%, leaving **~20–25% unattributed**. The mechanism: your canary sandwich samples a core that is *idle between samples*. An idle core **boosts** for an 8 ms sample even when the *loaded* core has been DVFS-capped by skin temperature for 60 seconds. The canary measures boost *capability*, not sustained *load* frequency. Conclusion: sustained single-core frequency on this SoC is ~0.72×, the "normalized 1.531" in the record is quarantined, and **the raw 1.562 is the only honest sustained number**. The fix is structural and it lands in Phase 3: the engine itself becomes the thermometer (below).
2. **Ladder ledger incomplete.** R2 (medium state layout), R3 (NEON tally), R4 (presieve {17,19}) have no recorded verdicts. This is not paperwork: those rungs pay **more on the A55 than the A76** (in-order cores suffer scalar tally and state-scan costs disproportionately) — and Phase 3 partitions six A55s. Record kept/reverted/not-run for each *before* Phase 3 weights are baked.
3. **The head-to-head isn't certified yet.** 2.346 vs 2.225 compares titan-optimized against primesieve-*default*. primesieve single-thread `--sieve-size=64` will land ~2.3–2.5. Run it under benchref, same session, same N (rate is N-dependent — your own sweep shows 2.83 B/s at 10⁹ vs 2.34 at 10¹⁰, partly medium-prime growth π(√N): 3.4K→9.6K→27.3K — so **all future rate comparisons pin N**). If primesieve-ST-best still loses, the claim is certified. If it doesn't, we know the true remaining gap before betting the MT phase on it.

Now the phase where the other 7 cores stop being spectators.

---

# Phase 3 Engineering Specification — `titan-pool` + MT titan-sieve: Heterogeneous Siege Engine

Phase 3's thesis, sharpened by every frozen measurement: **58% of this machine's sieve throughput lives on six little cores that don't throttle, and the enemy is thermal asymmetry.** The design that follows doesn't fight that asymmetry — it *recruits* it.

---

## PART 1 — MANDATE AND THE LAWS

**Scope:** `titan-pool` (worker spawn/pin/telemetry, unit pool, load distribution) + MT driver in titan-sieve + pre-flight experiment binaries + MT gate. **Excluded:** erat_big/buckets (Phase 4 — with the quantitative trigger now computable, Part 5), titan-count MT (Phase 5), any repinning/duty-cycling policy that lacks measurement backing (Part 6).

1. **Purity boundary holds.** Engine lib stays dependency-free; only binaries link titan-bench.
2. **Pin or report.** Every thread self-pins and *asserts* its CPU into telemetry. Unpinned measurement is forbidden — and the inherit-mask trap (Phase 0's most dangerous defect) dies permanently by assertion, not by vigilance.
3. **Zero locks in steady state.** All synchronization events are enumerated and counted (Part 7). A mutex on the marking path is a design failure, not an optimization miss.
4. **One thesis per experiment.** E1–E5 below each answer exactly one question; no experiment is trusted until its number reconciles with a model.
5. **Correctness precedes tuning.** Partition invariance green on scalar MT before any scheduling experiment runs.

---

## PART 2 — PRE-FLIGHT EXPERIMENTS (Data Before Design)

| # | Experiment | Question | Method | Feeds |
|---|---|---|---|---|
| **E1** | Per-core engine rates | What does *titan-sieve* (not the naive proxy) do on each core type? | Full survey pattern with the real engine, both segment geometries (64 KiB big / 32 KiB little), 10⁹ and 10¹⁰, canary sandwich, cool device | **The weight vector** — the single most important input to the partitioner |
| **E2** | Cool all-core burst | Contention factor for the real engine (L1-resident segments, streaming state — should beat the naive proxy's 80.1%) | 8 pinned workers, one barrier, cool, min-of-5 at 10¹⁰ | Weight derating + scaling prediction |
| **E3** | Core-mix sustained sweep | **The thermal thesis:** what core mix maximizes *equilibrium* throughput? Does dropping little cores buy back big-cluster clock? | Sustained 90 s runs for mixes {2 big + k little, k = 0..6}, engine telemetry as the instrument | Sustained scheduler policy; the winnable-lane data |
| **E4** | Single-core sustained telemetry | Resolve the 33% mystery: is the 10¹¹ single-core droop thermal (monotone curve) or memory (flat)? | 10¹¹ single-core, per-segment time series recorded | Patches Phase 2's record; proves telemetry-as-thermometer |
| **E5** | Bandwidth utilization | Is DRAM a co-limiter at 10¹¹ 8-worker? | Aggregate state-traffic math cross-checked against mem-canary-under-load | Part 5 budget; Phase 4 trigger data |

**E1 is the gate-opener for a hard reason:** the gate's headline target (beat 9.15 B/s) arithmetically requires 2×2.83 + 6×r₅₅ × contention ≥ 9.15 → **r₅₅ ≥ ~0.85 B/s**. If the A55 lands lower, R2/R3/R4 become prerequisites, not options — and if it lands *anomalously* low, suspect register spill in R1's 8-way unroll on the in-order core (check for an A55-tuned 4-way variant).

---

## PART 3 — THE ARCHITECTURE

### 3.1 Thread topology and the pinning protocol

8 compute lanes: 7 spawned workers + the main thread as worker-on-cpu7 (monitoring folded into its unit loop — a separate monitor thread would burn a core slot for ~0.0001% duty). Each worker's first act on spawn: **self-pin, assert success, publish CPU id to its telemetry slot, wait at the start barrier.** Self-pinning at spawn (rather than parent-pinning) sidesteps mask inheritance by construction; the affinity assertion in telemetry makes any regression loud. Pin failure = reported hole, never a silent skip.

### 3.2 Memory topology

| Region | Lifetime | Sharing | Notes |
|---|---|---|---|
| Base primes ≤ √N | construction | **read-only, all workers** | ~110 KiB at 10¹¹ — one copy, clean lines, SLC/L2-resident, zero coherence traffic |
| Presieve replica (66 KiB) + wheel tables | construction | **read-only, all workers** | same — 8 readers of one hot copy beats 8 private copies |
| Per-worker segment buffer | per worker | private | geometry per core type (Part 4) |
| Per-worker medium state | per worker, per unit | private | range-local, streamed sequentially |
| Unit array | construction | read-only after build | |
| Pool index | construction | **the one shared writable line** | |
| Telemetry slots | construction | one writer each, 128 B padded | main reads without locks; stale reads are harmless (advisory data) |

One construction-time arena; tripwire asserts zero thereafter with 8 workers live.

### 3.3 Work units

- **Geometry:** units are aligned to 64 KiB-byte boundaries (multiples of 30×65,536 = 1,966,080 numbers) so both 64 KiB and 32 KiB segment geometries subdivide any unit exactly. No cross-geometry alignment code exists.
- **Granularity:** ~128 units for 10¹¹ (~780M numbers each ≈ 280 ms big-core / ~700 ms little-core work). Unit init = one division per active prime ≈ 27K × ~15 cycles ≈ 0.2 ms (A76) — **0.07–0.25% overhead, provably noise.** This kills any temptation to build state-carryover machinery between units: fresh init per unit is simpler *and* free.
- **Front-load:** each worker's initial batch ∝ E1 weight × E2 contention factor; ~40% of units remain in the shared pool.

### 3.4 The pool — the central design idea

A single atomic index over the unit array. Worker loop: *pull unit (one `fetch_add`) → init → sieve segments → tally locally → publish telemetry → repeat → exit when pool empty and batch done → join.*

**Why this is the whole scheduler — the pool is an implicit thermal feedback controller.** Cool-state weights over-assign to big cores; when the big cluster derates (your Phase 0 split: big retains 36–76%, little retains 91–100%), big workers finish units slower and *pull less*; little workers — thermally stable — *absorb the remainder*. The imbalance corrects itself through the one shared line, with zero monitoring in the control loop, zero locks, zero policy code. The machine's thermal asymmetry, which kills naive 8-way splits, becomes the balancing mechanism itself. Telemetry (Part 6) *observes* this controller for the record and for future policy experiments; it is not in the loop. **Do not add anything to this loop.** Every proposed "smarter" scheduler must beat this one by more than its own overhead before existing.

---

## PART 4 — HETEROGENEOUS GEOMETRY

64 KiB segments on A76 workers, 32 KiB on A55 workers (L1D-matched — E1 confirms or corrects per type). Segment size is per-worker *runtime config*, Phase 2's law paying off. Consequences: each worker's tally is local (no cross-geometry aggregation issue — counts are counts); unit sub-segmentation is worker-internal; the per-geometry **sweep result from E1** becomes the per-cluster default with the same one-change-per-rung recording discipline.

---

## PART 5 — THE COST MODEL: EVERY CYCLE, ACCOUNTED

Cool-state steady cycle ledger, big core, 10¹¹ (must reconcile with E2 within 2× — same discipline as Phase 2's model calibration):

| Component | Cycles/number | Design consequence |
|---|---|---|
| EratSmall marking (R1 unrolled) | ~0.55 | the payload — already optimal-ascending, prefetch-perfect |
| Medium scan + walk | ~0.10–0.15 | 25,400 primes × 16 B ≈ 400 KiB state streamed per segment — sequential, prefetch-covered; R2's packing halves this if E5 says so |
| Presieve copy | ~0.01 | one linear 64 KiB copy from shared hot replica |
| Tally | ~0.02–0.05 | R3's NEON kernel matters *here*, mostly on A55 |
| End-masking, edges, activation frontier | ~0.01 | amortized O(1) |
| Unit boundary (init + atomic pull + telemetry store, amortized) | <0.005 | proven noise by the §3.3 arithmetic |
| Steady-state synchronization | ~0.00002 | ~8–16 shared-line transfers **per second** across all workers |
| **Total** | **~0.85–0.95** | matches Phase 2's measured 0.94 — the model closes |

**Bandwidth budget (E5):** per-worker state traffic ≈ 0.4–0.6 GB/s → aggregate ≈ 3.5–5 GB/s against ~8–10 GB/s effective LPDDR — a co-limiter at 40–50% utilization, safe at 10¹¹, *the* number to watch as rates climb.

**The certified MT domain stays 10¹¹ — now with the exact wall computed:** at 10¹², √N = 10⁶ exceeds the medium ceiling 8S ≈ 524K, stranding ~35,200 primes (π(10⁶) − π(524K)) in per-segment no-op scans: 35,200 × 508,650 segments ≈ 1.8 × 10¹⁰ wasted touches ≈ **40–50 s of pure bookkeeping**. That is Phase 4's bucket trigger, quantitatively frozen.

**The pre-cliff completion theorem — Phase 3's race condition:** the thermal cliff hits at t ≈ 14.5 s. 10¹¹ completes pre-cliff iff aggregate burst ≥ 10¹¹/14.5 ≈ **6.9 B/s**. Projection: 2×2.83 + 6×r₅₅ × E2-contention — with r₅₅ ≥ 0.85, that's ~10 B/s → **10¹¹ finishes in ~10 s, inside the cliff, where sustained throttling never engages.** The entire "sustained" battle at 10¹¹ may be won by simply being fast enough to dodge it. Sustained then only truly bites at 10¹²+ — Phase 4's war.

---

## PART 6 — THERMAL POLICY: TELEMETRY AS THERMOMETER, POOL AS CONTROLLER

Each worker writes (unit id, wall time, prime count) to its padded slot at unit boundaries; the main thread reads between its own units (~300–700 ms sampling resolution — ample for second-scale thermal). **This fixes the Phase 2 canary blindness structurally:** per-segment/per-unit times measured *on the loaded core, under load* are the one signal an idle-core canary can never see. E4's curve shape is the proof of instrument: monotone droop = thermal (expect ~0.72 confirmation); flat elevation = memory; spiky = interference.

Policy levels, shipped in order of evidence: **(a)** record-only — telemetry curve in every benchmark record, replacing canary normalization for sustained engine runs; **(b)** the pool — already the active controller, zero code; **(c)** core-parking / duty-cycling little workers to buy big-cluster clock — **forbidden until E3 shows the trade is real** (the arithmetic says little cores are 58% of throughput at ~3–4× better perf/W; sacrificing them must return more than it costs, and only the mix sweep can say). Speculative thermal cleverness without E3 data is how projects ship slower engines with prettier dashboards.

---

## PART 7 — THE SYNCHRONIZATION INVENTORY (Law 3, made exhaustive)

Total lock-free events per run: 1 start barrier, ~128 atomic pulls, ~128 unlocked telemetry writes (single-writer slots), 1 join. **Zero mutexes, zero condvars, zero atomics per segment, zero shared mutable state in any marking path.** The one contended cacheline moves ~a dozen times per second; false sharing is prevented by 128 B padding on all per-worker slots. This inventory is asserted in the gate — if a profiler ever shows the pool line hotter than this, something regressed.

---

## PART 8 — CORRECTNESS INSTRUMENTS (The New Failure Class: Seams)

MT introduces exactly one new mathematical failure mode: **partition boundaries.** π(N) = Σ π(unitᵢ) holds only if the cover is exact — an overlapping or gapped seam double-counts or drops primes *only at boundaries*, invisible to every single-thread test.

- **Partition invariance test:** randomized unit-grid covers of [2, N], k ∈ {1, 2, 4, 8} workers, **5 reruns with injected sleep-jitter to scramble pool pull order** — all must produce bit-identical totals (u64 addition commutes, so any order-dependence reveals hidden state), equal to single-thread π(N) and to oracle truth.
- **M-seam mutant:** deliberately overlap adjacent units by one number (or gap them) — the partition test must catch it at the first cover. If it survives, the test grows until it doesn't.
- **MT oracle:** batch streaming protocol against the MT engine at k = 1, 2, 4, 8 — full tiers, deep T3 at 10⁹/10¹⁰/10¹¹, randomized differentials vs primesieve **pushed to the domain edge [10¹⁰, 10¹¹]** (Phase 2's ran [10⁸, 5×10⁹]; the MT certification domain demands the upper edge).
- Affinity assertion (8 distinct CPUs in telemetry), zero-alloc tripwire with 8 live workers, hygiene gate + wake-lock, canary sandwich around whole runs (burst only — sustained normalization is telemetry's job now).

---

## PART 9 — BENCHMARK PROTOCOL AND THE GATE

Two columns, always, same-session references: **burst** (10¹⁰, min-of-5, pre-cliff) and **sustained** (duration-mode 90 s + full 10¹¹ run with telemetry curve). Scaling table k = 1..8 and the E3 mix table are deliverables, not extras. primesieve best-config ST head-to-head (audit item) and best-config 8T burst/sustained run in-session — we beat its *best*, at pinned N.

| # | Criterion |
|---|---|
| 1 | E1–E5 records exist and reconcile with their models (E4 curve classified: thermal/memory/interference) |
| 2 | Phase 2 ladder completed: R2/R3/R4 verdicts recorded; primesieve ST best-config head-to-head recorded, claim certified or corrected |
| 3 | Oracle full mode, MT engine, k ∈ {1,2,4,8}: **exit 0**, all tiers + upper-edge differentials |
| 4 | Partition invariance: 5 jittered orderings, k-sweep, bit-identical, equals single-thread and oracle |
| 5 | M-seam mutant killed by the partition test |
| 6 | Affinity assertion: 8 distinct CPUs in telemetry; zero-alloc tripwire green with 8 workers |
| 7 | Sync inventory asserted (no locks in steady state; pool-line traffic within budget) |
| 8 | **Burst aggregate ≥ 9.15 B/s** at 10¹⁰ (8 workers, vs primesieve best-config, same session) |
| 9 | **Sustained ≥ primesieve best-config sustained** (duration mode, same session); telemetry curve recorded |
| 10 | 10¹¹ pre-cliff completion attempted; time-to-complete recorded vs the 14.5 s cliff |
| 11 | Scaling table (1..8) + E3 mix table recorded; cycle ledger reconciled within 2× (E2) |
| 12 | Gate record persisted; reference.md v3 with both columns and per-worker telemetry |

---

## PART 10 — SPEC AMENDMENTS AND THE DECISION MAP

**Amendments (file_structure.md v2.2):** (1) canary normalization declared invalid for sustained engine runs — replaced by engine telemetry; Phase 2's sustained normalized column patched; (2) MT certified domain = 10¹¹, with the 1.8 × 10¹⁰ no-op-touch derivation recorded as Phase 4's trigger; (3) pool-as-thermal-controller documented as the standing load-balancing mechanism — additions require beating it by more than their overhead; (4) per-worker heterogeneous segment geometry as runtime law; (5) all rate claims pin N.

**Decision map:** E1 weights + E3 mix table → Phase 4's scheduler inputs and any policy-(c) decision; E5 bandwidth + medium-state scan cost → R2 packing priority and Phase 4's bucket threshold; telemetry framework → every benchmark in every future phase (and the primecount-side too, eventually); the certified MT pi_range engine → **Phase 5's P₂ term verbatim** (Gourdon/Lehmer's P₂ is literally "count primes in (x/p, x/q] ranges" — the same machinery, second consumer); partition-invariance harness → the permanent MT merge gate.

---

The single-thread war is won or certified-winnable; Phase 3's war is against a thermal enemy that punishes symmetry. The design above doesn't schedule *around* the asymmetry — it makes the slow-heating cores the ballast, the pool the autopilot, and every cycle either marks, counts, or gets amortized to a rounding error. Run E1 first and paste the A55 number — it alone decides whether the gate opens at 9.15 or whether the unreported rungs get their day first.
