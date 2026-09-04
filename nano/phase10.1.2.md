# Phase 0 — Engineering Spec

Phase 0 exists to produce seven artifacts and four decisions. Every later phase consumes exactly one of these artifacts; nothing downstream may proceed on an unmeasured number. Dependency map first, so the order of sessions is forced, not chosen:

| Artifact (this phase) | Gates (later) |
|---|---|
| Runner + session statistics | every claim, phases 1–9 |
| `device_profile.json` | segment size classes (P4, P7, 9.5) |
| Anchors + golden partials | every build change (Law 2) |
| Thermal constants | rig validity itself |
| `baseline_fair.json` | the ledger's "now" row |
| `kappa_table.csv` | prices P7, P9; decides if 9.5 lives or dies |
| `enum_results.json` | pinned config adoption; freebie |

Sessions run S0→S6 in order because each consumes the previous one's output.

---

## S0 — Runner binary and silicon probe (½ day)

**Runner.** A small on-device binary (Rust, same toolchain as Titan). It is the only thing that ever launches a timed run. Spec:

- Session config file: list of runs `{label, argv, env, cooldown}`, pair structure, pair count N=24, discard-first-k=3, cooldowns, RNG seed.
- Launch: `fork`/`exec` + `waitpid`, wall from `CLOCK_MONOTONIC` around the child. Never shell `date`, never in-process self-timing for the wall claim.
- **Per-run output check:** parse the child's printed π(x), compare against the anchor fixture. Mismatch → abort session immediately. A wrong π is a code or env bug, not noise; it must never enter a median. This is Law 2 applied to the measurement apparatus.
- Self-pin the runner to an A55 outside the current measurement's CPU set (fallback: an A78, logged) so the logger doesn't share a core with a 1-thread cell.
- Freq logger thread: read `/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq` every 100ms; thermal zones every 1s. Attach to session log. Freq logs are the primary throttle detector — thermal zones are a bonus if readable.
- Record in every session JSON: git SHA of Titan, SHA of the oracle build, sha256 of the opponent binary, battery %, airplane-mode on, screen off, not charging, seed, loadavg at session start.

**Silicon probe.** Read, don't assume:

- CPU inventory from `/sys/devices/system/cpu/{present,possible}`; expect 8.
- Class map from `cpuinfo_max_freq` per cpu (≥2.0GHz → A78). Expect 2×A78 + 6×A55. If the read disagrees with 2+6, stop and re-map — every downstream constant depends on it.
- Caches: per cpu, per index: `level`, `type`, `size`, `shared_cpu_list`. Output: L1D size per class (expect 32KB/64KB — verify), L2 sharing structure (which cores share which L2), whether any L3 exists (expect none).
- Governor per policy: `scaling_governor`, `scaling_available_governors`. Record as **uncontrollable without root** — this is why every κ number carries its freq log: schedutil ramp and clamping are part of the measured reality.
- Topology: `thread_siblings` — confirm no SMT; if pairs appear, the CPU-count arithmetic in everything below changes.

Write `device_profile.json`. Done when a fresh process can answer: cpu→class, cpu→L1D size, cpu→L2 set.

## S1 — Anchors, oracle build, golden partials (½ day)

**Anchor fixture.** Script extracts π(10^k), k=12…18 constants from the tree's own source files, emits `fixtures/pi.toml`. The doc and every gate cite the fixture, never a typed digit. The script is idempotent; CI re-runs it and diffs. A dropped digit can never again survive to a gate.

**Oracle build.** His source, our instrumentation, compiled exactly like the opponent (`-O3`, no arch flags) so κ measured on it reflects shipped behavior. Instrumentation, all boundary-only (zero hot-path cost):

1. Region timers: `clock_gettime` around each named OpenMP region → dump `{region, wall, threads, placement}`.
2. Setup/region split: clocks around table construction vs region body. **This is the Phase-5a measurement delivered now** — "how much of the 3-region cost is table compute vs pool mechanics" gets answered by S1 output, and 5a's "measure, don't assume" is retired before Phase 5 exists.
3. Pin shim: env `PC_PIN="B:6,0,1;D:7,2,3,4,5;AC:..."`. At region entry, thread i pins itself to spec[i], then **reads back its own affinity and asserts**. A silently failed pin poisons every routing decision downstream; readback-and-abort is the only acceptable failure mode. Placement dumped per region.
4. Thread-count hooks: `PC_THREADS_B`, `PC_THREADS_D` override the internal region thread counts. Behavior change certified the cheap way (below).
5. `pc_noop_region(threads, iters)`: empty parallel region looped 100× — isolates pool wake + barrier cost as a number. Replaces the deleted FFI parenthetical with a measurement.
6. Per-b partial dump, env-gated, 1-thread runs only.

**Golden partials.** Run each term at 1 thread, x=1e16, pinned, instrumented dump → freeze as golden files. Rules:

- Any build change (phases 1–9) re-runs the 1T cells and must bit-match golden before any wall measurement is taken.
- Multi-thread runs certify at sum level only (per-b totals), because his dynamic balancer makes per-thread attribution non-deterministic. 1T is the deterministic reference; sums are the invariant at all thread counts.
- The 1T κ cells in S5 double as re-certification runs — every κ session re-proves the oracle for free.

## S2 — Thermal study (25 min machine, sets rig constants)

Purpose: pick cooldowns empirically so that per-run wall time is stationary across a session (no monotone accumulation), which is what makes paired medians valid.

Protocol: 20 Titan runs, 0s cooldown → fit wall vs run index, record %/run slope. Repeat 12 runs at 10s, 20s, 30s cooldowns. Accept the smallest cooldown where |slope| < 0.5%/run. Then log temperature until return-to-baseline (capped 10 min) → sets inter-session spacing. Also inspect the within-run freq trace: if a 3s full-load burst already clamps A78s, all runs clamp equally and that's a stationary, acceptable state — record it.

Initial guesses to be replaced by measurement: inter-run 15s, inter-pair extra 15s, sessions same time of day.

Output: `thermal_profile.md` with the constants the runner will use.

## S3 — Baseline fair session + rig reliability (2 × 18 min, two mornings)

First fair measurement of Titan vs primecount. Protocol, fixed and pre-registered:

- 24 pairs. Pair i = one `primecount 1e16` run + one Titan run, order decided by recorded-seed coin flip per pair. Fixed cooldowns from S2. First 3 pairs discarded.
- d_i = t_Titan − t_pc within pair. Report: median(d), MAD(d) = median |d_i − med(d)|, sign fraction, per-order medians (pc-first subset vs Titan-first subset), per-binary p50, freq summary per class.
- **Claim gate (this is the permanent gate for every future claim):** |median(d)| > 2·MAD(d) **and** sign consistency ≥ 15/21 kept pairs.
- **Session abort:** MAD(d) > 0.25s → too hot, discard session, reschedule. Weather, not code.
- Anti-p-hacking: N, seed, gates written to the session config before launch. No reruns because a result looked off; only full-session aborts. No extending N after looking.

Titan's internal term dump is collected per run; the decomposition table (your session table) is re-baselined under fair protocol. Expectation to verify, not assume: his 2.96 was cold-gifted, so his fair median likely rises — the true gap is probably larger than 0.3–0.5s, and Phase 0 is where that gets measured.

External/internal cross-check: runner wall vs Titan's Σ(terms) + overhead; discrepancy > 3% is an accounting bug flag, investigated before any number is quoted.

**Rig reliability:** repeat the identical session on a different day. Require |med₁ − med₂| ≤ 2·max(MAD₁, MAD₂). If this fails, the rig is not a measurement device yet — fix cooldowns or session timing before anything else in the plan claims a result.

## S4 — κ table (8 min machine)

Goal: per-kernel, per-class rates through the real code paths. Mechanism: one fresh process per cell (no pool contamination), 1 warmup iteration (frequency settles, pages touched), 5 timed iterations, median + MAD, freq log attached, pin readback verified. Instrumented build dumps region-only time (setup split out), so κ measures kernel throughput, not per-call table construction.

Grid (all at x=1e16):

| kernel | A55 thread counts | A78 thread counts |
|---|---|---|
| B (FFI, his code) | 1, 2, 3, 4, 6 | 1, 2 |
| D (FFI, his code) | 1, 2, 3, 4, 6 | 1, 2 |
| AC (native, ours) | 1, 2, 4, 6 | 1, 2 |
| C1 (ours, serial today) | 1 | 1 |

Plus: no-op region cell; and a spot check — B and D at 1 thread per class at x=1e14 and 1e17, to detect whether κ drifts with scale (if it does, routing tables are per-scale, which changes Phase 7/9 design now, cheaply).

Derived from the grid:

- κ_k = wall_1T(A55) / wall_1T(A78) per kernel — at observed clock and normalized to equal clock (both reported: observed is what scheduling sees; normalized reveals microarch structure).
- Scaling curves per class (does D actually scale to 6 A55s, or saturate on the atomic balancer / bandwidth? non-monotone cells are recorded, not smoothed — the routing model uses measured points only).
- The two-column ledger: W_k in A55-seconds and A78-seconds. Note the caveat honestly: work *counts* are class-invariant, but rates are not, and cache effects sit inside the rate — which is fine, because the enumeration in S5 measures actual walls directly; the model only prunes candidates.

C1's single row is the first divergent-code data point (recursive, branchy → if κ_C1 is the largest κ in the table, the per-core-class assignment thesis has its first evidence; if it's flat, that's evidence against, recorded either way).

**Decision D1 (written the same day):** differential κ = max spread of κ across {B, D, AC, C1}. Spread ≥ 1.3× → Phase 7 and 9.5 are real, priced by these numbers. Spread < 1.1× → cut 9.5 and Phase 7's headline from the plan; scheduling upside on this chip is ~zero and we know it in week one.

## S5 — Enumeration (35 min machine, only if κ > 1 materially)

The arithmetic that generates the candidates, stated so it can be checked:

B+D overlap on 6 A55 + 2 A78. Effective speed of a term's thread set = n_A55 + κ·n_A78 (A55-units). Wall = max over the two terms of W_term / speed_term. Total bound: wall ≥ (W_B + W_D)/(6 + 2κ) — with the fitted 5.5 + 9.0 = 14.5 A55·s, κ=1 gives 1.81s (exactly current), κ=1.5 gives 1.61s. **The freebie exists iff κ > 1 — the κ table decides whether this session runs at all.** If κ is flat, run only E6 as a sanity check and stop.

Non-obvious consequence of the max structure, derived not vibes: with two concurrent terms and only 2 A78s, **balanced assignment (one A78 to B, one to D) beats pole-feeding (both A78s to B)** — giving B both big cores makes D the new pole at 9/5 = 1.8s and buys nothing. The optimum sits near the point where both terms' effective speeds equalize. Candidates are the integer points around that equalizer:

- E0 — control: current unpinned default.
- E1 — balanced: B = {A78, A55, A55}, D = {A78, 5×A55}.
- E2 — pole-feed: B = {A78, A78, A55}, D = 5×A55. (Model says no gain; included to verify the model's failure mode.)
- E3 — balanced + thread shift: B 4T {A78 + 3×A55}, D 4T {A78 + 3×A55}.
- E4 — falsification: both A78s to D. Model says worse. If it wins, the model is wrong and that finding blocks all routing work until explained.
- E5 — best of E1–E3 with AC pinned too (AC runs sequentially before B/D, so its pinning is independent — enumerate orthogonally, compose the winner).
- E6 — `OMP_PROC_BIND=spread` alone, no shim. If E6 ≈ E1, the shim wasn't needed; if E6 ≈ E0, EAS/libgomp defaults are confirmed neutral.

Protocol per candidate: 20 pairs vs E0 (both runs are Titan, one with env pins, order randomized per pair, recorded seed), same claim gate as S3: |median(d)| > 2·MAD(d) with d = pinned − default. Adoption rule: best gate-passing candidate wins; ties → simpler config. Adopted config becomes the new default and gets its own fair session vs primecount → new `baseline_fair.json`, new ledger "now" row.

Every candidate's π output is anchor-checked per run; every run's pin spec is readback-verified in the dump.

## Failure modes and dispositions

| Symptom | Cause | Action |
|---|---|---|
| Pin readback mismatch | shim bug / cpu id off | cell invalid, fix before continuing — never run on |
| Anchor mismatch on any run | code/env corruption | abort session, find the bug, restart session |
| MAD(d) > 0.25s | ambient/thermal | discard session, reschedule cooler |
| Per-order medians differ by > MAD | cooldown too short, order effect real | raise cooldown from S2 data, re-run session |
| Two-day medians disagree | rig not stationary | fix rig before ANY phase claims a win |
| κ non-monotone in threads | real serial fraction / contention | keep raw points; routing uses measured walls only |
| External wall ≠ internal Σ by > 3% | accounting bug | resolve before quoting either number |
| A78 freq clamped > 10% mid-session | thermal | visible in freq log; session flagged, medians suspect |

## Schedule and Definition of Done

| Session | Work | Machine time |
|---|---|---|
| S0 | runner + probe | 30 min |
| S1 | oracle + anchors + golden | 15 min |
| S2 | thermal study | 25 min |
| S3 | baseline × 2 days | 2 × 18 min |
| S4 | κ grid | 8 min |
| S5 | enumeration | ~35 min |

≈ 2–2.5 days wall-clock. Phase 1 templating can be coded in parallel after S3 — it doesn't need the rig until certification.

Phase 0 is **done** when `decisions.md` contains, each with its measured number:

1. Rig reproducibility passed across two days.
2. Fair-protocol opponent median and per-term re-baseline (the ledger's real starting line).
3. κ table + D1 verdict: differential spread number, and 9.5/Phase-7 live-or-cut.
4. Enumeration verdict: adopted config or "no freebie, κ flat", with its session JSON.
5. Region/setup split from the oracle (Phase 5a's answer, already banked).

The next artifact that matters is `kappa_table.csv` — it prices Phases 7 and 9 before either is attempted, and it's eight minutes of machine time away.
