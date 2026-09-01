# Phase 3 Post-Game Audit — Then Phase 4: The Deep Domain

Read the report with the gate table side-by-side, because Phase 3 delivered two different things and only one of them is finished.

**What is genuinely certified:** the architecture. Partition invariance bit-identical across k, M-seam killed, 8 distinct pinned CPUs asserted, zero allocs with 8 live workers, exact π(10¹¹), and the pool balancing a throttling heterogeneous SoC to ≤3% deviation with no locks and no monitor thread — that last number is the phase's real trophy. The implicit thermal controller works exactly as derived.

**What is not certified — the performance half of the gate:**

| Gate item (Phase 3 spec) | Target | Measured | Status |
|---|---|---|---|
| 8. Burst aggregate at 10¹⁰ | ≥ 9.15 B/s | 6.172 B/s (67%) | **NOT MET** |
| 9. Sustained ≥ primesieve best-config | ~4.2 B/s (their 10¹¹) | 2.428 B/s (57%) | **NOT MET** |
| 10. Pre-cliff completion analysis | recorded | absent | **OWED** |
| E3 mix sweep, E4 single-core curve, E5 bandwidth | — | not run | **OWED** |
| R2/R3/R4 ladder verdicts | recorded | not run | **OWED** (carried from Phase 2) |
| primesieve ST-best head-to-head, same session | recorded | absent | **OWED** |

None of this is hidden — your own scaling table shows it — but the ledger discipline says we name it: **correctness green, architecture green, throughput at 57–67% of the summit.** The sustained 2.428 B/s raw number is honest and stands (telemetry-thermometer policy from Phase 3 is correct; the quarantined number is only the canary-normalized column, which the engine's own telemetry now supersedes).

**The diagnostic that defines Phase 4 — the E2 inversion.** The naive proxy contended at 80.1% (Phase 0). The real engine contends at 66.5%. The engine is *faster* per core and contends *worse* — that inversion is not a paradox, it's the fingerprint of a memory wall: the proxy's state traffic was a ~100 KB cached base-prime list; titan-sieve streams ~277 KiB of medium state per segment per worker. Run the arithmetic on your own numbers: under 8T, each A76 retained **94%** of solo rate (2.2 of 2.333) while each A55 collapsed to **38%** (0.295 of 0.768). Six in-order cores with weak memory-level parallelism, sharing one cluster L2 (~256 KiB) that a 6 × 277 KiB working set obliterates, queued behind two out-of-order cores with deep load windows — the little cluster is *latency-starved*, not bandwidth-starved. 58% of the machine's capacity is being throttled by its own state format. That is the Phase 4 thesis, and it is measured, not speculated: **fix the bytes, and the cycles follow.**

---

# Phase 4 Engineering Specification — `erat_big` + Bucket Architecture + The Memory Attack

**Mandate:** extend the certified domain from 10¹¹ to 10¹² (live-gated), 10¹³ (marathon-certified), 10¹⁴ (optional overnight badge); close the Phase 2/3 memory debts; make the engine survivable on a phone for hour-scale runs. New character: up to now every cycle was accounted; from here **every byte is accounted with equal rigor** — at deep N, DRAM traffic, not instructions, is the ceiling.

---

## PART 1 — LAWS OF PHASE 4

1. **The memory law.** Every design change reports Δcycles/number *and* ΔDRAM-bytes/number. A change that buys cycles with bytes is presumed guilty at multi-core until F1 says otherwise.
2. **The machinery-visibility law.** New deep-domain machinery (buckets, carry lists, checkpoints) must be certified *exhaustively at a forced configuration where it is fully active at small N* before it is trusted at scale (Part 7). You do not verify a 10¹³-only code path by only running 10¹³.
3. **The crash law.** Any run over 5 minutes is checkpoint-resumable; kill-and-resume bit-exactness is a permanent gate criterion. A phone OS that can kill you at any moment is part of the spec, not an environment quirk.
4. **The tier-boundary law.** Boundaries are derived quantities of (S, memory economics), per-worker, not global constants.
5. **Pool law unchanged.** Buckets are strictly per-worker; the atomic pool remains the only shared structure. The sync inventory does not grow.

---

## PART 2 — THE DOMAIN MATH: WHY BUCKETS ARE FORCED, AND WHAT THEY COST

At N = 10¹², √N = 10⁶. The medium scan tier as-built covers (S/4, 8S] = (16,384, 524,288] — but π(10⁶) = 78,498 sieving primes exist. The 35,500 primes above 524,288 would be *scanned per segment for nothing* most of the time: 10¹² / 1.966M ≈ 508,650 segments × 35,500 no-op touches ≈ 1.8 × 10¹⁰ wasted loads — tens of seconds of pure bookkeeping. At 10¹³ it's 9.4 × 10¹¹ touches — **hours**. This is the Phase 3-derived trigger, now exact.

The bucket principle: a prime p > boundary has at most one crossing per segment and often zero. So never *scan* for its crossings — *schedule* them. Each prime's next crossing is stored as an entry in a bucket keyed by the segment it will land in; when the sieve reaches segment s it drains only bucket[s], and each drained entry immediately computes its next crossing and re-pushes itself into the future bucket it belongs to. A prime that skips 500 segments costs zero during those 500 segments.

**The invariant cost (this is the law to internalize):** bucketed sieving costs ~2 × entry-size bytes of DRAM traffic *per bucketed crossing* — write once at push, read once at drain. With 8-byte entries that is 16 B per crossing, and crossings are a Mertens-density fact you cannot engineer away. The per-number bucket traffic is therefore:

| N | bucketed crossings/number | traffic (8 B entries) | traffic (6 B entries) |
|---|---|---|---|
| 10¹² | 0.013 | 0.21 B/num | 0.16 |
| 10¹³ | 0.049 | 0.78 B/num | 0.59 |
| 10¹⁴ | 0.054 | 0.86 B/num | 0.65 |

Every other stream (medium state, presieve from the shared SLC replica, carry list) sums to ~0.2–0.3 B/number more. So the engine's total DRAM demand runs **~0.5 B/number at 10¹², ~1.0 at 10¹³** — nearly scale-invariant, growing only double-logarithmically. Against an effective ~5–6 GB/s LPDDR, that sets the multi-core cool ceiling: **~9–10 B/s at 10¹², ~5.5–6.5 B/s at 10¹³.** Write this down as the asymptotic law: *past 10¹² the engine is DRAM-bound at full occupancy, thermal-bound at sustained occupancy — and those are different binding constraints requiring different optimizations.* The F1 experiment measures the real knee.

---

## PART 3 — TIER RE-DERIVATION (The Boundary Is a Memory-Cycle Tradeoff, and Packing Constraints Vote)

Two forces pick the medium→bucket boundary:

- **Traffic break-even:** scanning costs ~state-bytes per prime per segment; bucketing costs ~16 B per crossing. A prime crossing the segment c times costs 24 B/c by scan (24 B state, §Part 4) vs 16 B by bucket. Scan wins while c ≥ ~2 → boundary ≈ **4S** (crossings/segment = 8S/p, equals 2 at p = 4S).
- **Cycle no-op threshold:** below ~1 crossing/segment, scanning becomes pure no-op loads → boundary ≈ **8S**.

Plus one packing constraint: the scan-tier per-prime delta table in u16 is safe only for Δ ≤ 65,535, i.e. p ≤ ~330K ≈ 4S. **The traffic optimum and the u16 constraint agree: default boundary = 4S** (= 262,144 at S = 64 KiB; per-worker, derived from that worker's own S — the A55's 32 KiB geometry gets its own boundaries automatically). The 8S hybrid (u16 tables below 330K, multiply-walk above) is an experimental rung, not the default.

Resulting tier census — and the law worth framing:

| Tier | Range (64 KiB) | Primes | State |
|---|---|---|---|
| Small (R1 unrolled) | ≤ 16,384 | 1,900 | registers |
| Medium scan | (16,384, 262,144] | 21,100 | **N-independent** |
| Bucket | (262,144, √N] | 35.5K @10¹² / 205K @10¹³ / 621K @10¹⁴ | grows with N |

**The medium scan tier is O(1) in N.** All N-growth is absorbed by the bucket system, whose per-number cost grows double-logarithmically. That is the scaling architecture in one sentence.

---

## PART 4 — THE MEMORY ATTACK: R2 AS THE PREREQUISITE (Not an Optional Rung)

The E2 inversion diagnosis dictates the medium-state redesign, and it splits along a line nobody exploited before now — **constants vs. mutable:**

- **Per-prime constants:** the 8-entry Δ table (u16 × 8 = 16 B) and the wheel row. Pure functions of the prime. **Shared, read-only, one copy for all 8 workers, SLC-resident.** Six A55s stop streaming six private copies of the same data through one L2.
- **Per-prime mutable:** next-crossing offset + wheel index j — [offset | j] in 4–8 B, per worker, private. Store the offset **absolute in unit space** (not per-segment-relative), so non-crossing entries are read but never written, and the scan is a pure sequential stream — prefetch-perfect, which is exactly what latency-starved in-order cores need.

Per-worker mutable footprint: 21,100 × 4–8 B = **84–168 KiB** (vs 277 KiB today, private). Total state *traffic* falls ~4–5×, the shared constant table (336 KiB) moves to SLC where it is one clean copy, and the A55 cluster's L2 miss stream shrinks proportionally. This is the single change that attacks the 38% little-core collapse, the 10¹⁰/10¹¹ MT shortfall, *and* the deep-N medium economics — one design, three debts.

The tradeoff to measure (F5): Δ-from-table (16 B shared, ~2-cycle step) vs Δ-recomputed (0 B, one multiply-add per step, ~4–5 cycles). On the bandwidth-bound A55 cluster and at MT occupancy, the bytes likely win; single-core on A76, cycles likely win. **Per-cluster state strategy is legal** (tier machinery is per-worker; partition invariance guarantees the sum). Measure, then choose per cluster.

---

## PART 5 — THE BUCKET ARCHITECTURE

**Ring of segment buckets, W deep (default W = 16).** Buckets are indexed (segment mod W); a bucket is a singly-linked list of fixed-size blocks (512 entries) allocated from a pre-allocated per-worker pool — construction-time arena, zero runtime alloc, pool exhaustion is a loud telemetry abort (2× headroom over mean; crossings-per-segment is a 10⁵-event law-of-large-numbers quantity, ±1% — 2× is paranoia, and paranoia is cheap).

**Entry format — 8 B default, u64-packed:** [prime:24 (√10¹⁴ = 10⁷ < 2²⁴ ✓) | rel-in-target-segment:17 | j:3 | row:3 | bit:3] = 50 bits. Storing `row` (p mod 30's bit index) in the entry keeps the drain loop free of even a constant-divide; `rel` is relative to the entry's *target* segment so drain needs no rebasing. All segment arithmetic is shifts/masks because S is a power of two — that power-of-two law now pays for itself a second time. The 6 B u48 variant is a measured rung (burst headroom per Part 2), not the default: unaligned packed stores cost complexity that must buy measured rate.

**Drain (segment s):** iterate bucket[s mod W] sequentially; per entry: clear the bit in the L1-resident segment (scattered byte RMW in L1 — cheap, this is the design's beauty); compute next crossing (table lookups + one multiply-add); if it lands within s..s+W−1 push to that ring slot (same-segment re-push legal for p < 8S — the block-list drain consumes its own appends); else append to the **carry list**. ~6–8 cycles per entry, ~25% of segment cost at 10¹³ — measured via telemetry counters.

**Carry/fill:** entries whose next crossing exceeds the window roll into the carry list; at window rollover, each carry entry is re-slotted into the new window's ring (multiply-add + push). Per window the carry list is ~n_big entries — 205K at 10¹³ — amortizing to ~0.02–0.03 cycles/number. **Steady state contains zero divisions**: the only divisions in the entire deep path are at unit/range initialization (one per bucket prime, ~3M cycles per 78-billion-number unit — 0.00004 cycles/number, provably noise, same arithmetic that closed this question in Phase 3).

**Activation:** the p² frontier from Phase 2 generalizes — bucket primes activate at p² when it enters range, or at unit start via one ceil-division + coprime advance. Frontiers advance monotonically; O(1)-amortized.

**Per-worker pool sizing (W = 16, 8 B entries):** 10¹²: ~3.3 MB; 10¹³: ~8.7 MB; 10¹⁴: ~13.6 MB. ×8 workers: 26 / 69 / 109 MB — inside an 8 GB phone, sized from config at construction, asserted in the gate.

---

## PART 6 — CHECKPOINT / RESUME (The Phone-Native Systems Requirement)

An hour-scale run on Android *will* be interrupted — OOM killer, battery, user. primesieve never had to solve this; we do, and it's a differentiator no desktop engine has.

**Design: checkpoint at unit granularity only.** Never mid-segment, never mid-bucket — because unit re-initialization is exact and nearly free (the division-per-prime arithmetic above). Checkpoint state = {config digest, pool index, per-worker completed-unit list + partial counts} — hundreds of bytes. Durability: atomic-rename writes every 30 s (write temp, fsync, rename); up to 30 s of work is sacrificed on crash and recomputed idempotently on resume. Resume: rebuild pool from completed set, re-dispatch, continue. pi_range exactness is the correctness backbone — the same contract Phase 3's partition invariance already certifies.

**Policy split for the marathon:** correctness marathons (10¹³/10¹⁴) may run on charger (flagged in the record; excluded from rate claims — the hygiene law's thermal reasoning is about *performance validity*, not truth). Performance benchmarks stay unplugged. Both recorded, never confused.

---

## PART 7 — CORRECTNESS INSTRUMENTS: The Forced-Machinery Trick (Phase 4's Star Instrument)

The new failure classes are all *scheduling* losses: a dropped re-push (prime stops crossing → composites survive → **overcount**), a carry entry lost at window rollover (same), a ring-slot off-by-one (crossings applied to the wrong segment — over *or* under). None of them are reachable by the existing 10⁷ enumeration audit *at production geometry*, because buckets only engage above 10¹². The solution is to **shrink the geometry until the deep machinery is fully active at exhaustively-auditable scale:**

**Forced-bucket config:** S = 256 B (tier boundaries: small ≤ 64, medium (64, 1024], bucket (1024, √N]), W = 4 (and W = 2 for window-edge stress). At N = 10⁷: 274 bucket primes, every ring/carry/re-push path executes ~10⁵ times; at N = 10⁸: 1,057 bucket primes. Then run the *existing* instruments against it: **full enumeration audit to 10⁷/10⁸ element-wise** (the strongest instrument in the project — compensating count-preserving corruption dies here), pi_range invariance, the N mod 30 matrix, T1/T2 via the batch protocol. The deep machinery is certified where you can see every byte of it, then trusted at scale via milestone constants.

**Local mutant corpus:** M-bucket (drop every 1000th re-push), M-carry (drop every 1000th carry entry), M-ring (off-by-one slot on window wrap) — each must be caught by the forced-bucket enumeration audit; each kill recorded with tier. **M-checkpoint:** corrupt one checkpoint field (checksum must reject; tamper-detection is part of the crash contract).

**Crash gauntlet:** 20 × kill -9 at randomized times during a 10¹² run → every resume bit-exact → final π(10¹²) exact. This is the gate's most adversarial test and the one no reference engine has an equivalent of.

**Oracle extension — the certification tier split.** Live tier (every full gate): T1/T2/T3 through 10¹² live (π(10¹²) = 37,607,912,018 — a ~6-minute candidate run inside the gate budget), randomized differentials vs primecount at 5 points in [10¹¹, 10¹²] (primecount answers any x in ~0.1 s — instant truth at any scale; the sieve query is the slow side). Marathon tier: 10¹³ = 346,065,536,839 via the **cert-record** protocol — the deep gate binary writes a record (x, π(x), wall, config digest, telemetry digest, checkpoint count, charging flag, epoch range); the oracle gains a record-validation mode (constants check + field completeness + live spot-checks at smaller x). One full live 10¹³ run per release; records thereafter. 10¹⁴ = 3,204,941,750,802, optional overnight badge, same protocol.

---

## PART 8 — PRE-FLIGHT EXPERIMENTS (F-Series; E-Debts Folded In)

| # | Experiment | Question | Method | Feeds |
|---|---|---|---|---|
| **F1** | DRAM knee curve | Effective aggregate bandwidth vs worker count under the *real* access mix (sequential u32 streams + scattered 8 B pushes + L1 RMW) | Synthesized loads matching measured mix; sweep 1..8 workers | Entry-size decision (8 vs 6 B); the Part 2 ceiling validation; **the never-run E5, finally with the real pattern** |
| **F2** | Entry layout A/B | u64-packed vs SoA columns vs u48; drain-sort on/off, per core type | Forced-bucket config (cycles visible in seconds) + production spot-check | Drain loop design; sort verdict (predict: off on A76, maybe on on A55) |
| **F3** | Carry vs re-scan fill | Architecture B (persistent re-push) vs A (per-window division scan) | Forced-bucket config | Fill design confirmation; division budget |
| **F4** | W sweep | Window 8/16/32/64 at 10¹² | deep_bench | Fill overhead vs pool memory vs drain locality |
| **F5** | State strategy per cluster | Δ-tables (shared 16 B) vs recompute (multiply); 4S vs 8S hybrid boundary | Per-cluster A/B at 10¹² and 10¹⁰ | The R2 final form; tier knob default |
| **F6** | A55 deep geometry | 16 vs 32 KiB segments for bucket-heavy work; heterogeneous boundaries | A55-pinned deep_bench | Little-cluster recovery beyond R2 |
| **E3-debt** | Sustained core-mix sweep | Which mix maximizes *equilibrium* rate (2B+kL, k = 0..6) | 90 s real-workload runs | Marathon worker-mix policy |
| **E4-debt** | Single-core sustained curve | Classify the 10¹¹ single-core droop (thermal vs memory) | Telemetry from the 10¹² single-core run | Closes the Phase 2 33%-mystery file |

F1 runs first — it prices the byte in cycles for every decision after it.

---

## PART 9 — THE LEDGERS (Every Cycle *and* Every Byte, at 10¹³, A76, per Number)

| Component | Cycles/num | DRAM B/num |
|---|---|---|
| Small-tier crossings (0.355 × ~1.4, R1) | 0.50 | ~0 |
| Medium scan (21.1K loads) + crossings (0.067 × ~2.5) | 0.22 | 0.15–0.20 (mutable stream) |
| Bucket drain (0.049 entries × ~7) | 0.25 | 0.78 (push+drain) |
| Carry/fill amortized | 0.03 | 0.08 |
| Presieve copy / tally / masks / pool | 0.05 | ~0.03 (SLC-served) |
| **Total** | **~1.05** | **~1.05** |

→ Single A76 ≈ 2.1 B/s at 10¹³ (vs 2.34 at 10¹⁰ — the crossing-density tax, quantified). 8-core: cool burst **DRAM-bound ~5.5–6.5 B/s**; sustained **thermal-bound ~2.5–3.0 B/s**. Predictions to calibrate against — a measured 8T burst above 6.5 at 10¹³ means F1 under-measured the knee; below 4.5 means a ledger component is lying, and the per-tier telemetry counters (segments, entries drained, carry ops, scans — all cheap u64 increments per worker) say which one.

---

## PART 10 — THE LADDER (One Change Per Rung, Keep-or-Revert at ±3%, Oracle-Quick Between Rungs)

**G0** forced-bucket certification suite green (correctness first, always) → **G1** production buckets, 8 B entries, W=16, 4S boundary, R2a state — first 10¹² measurement, model calibration → **G2** R2 SoA constant/mutable split — *re-benchmark 10¹⁰ and 10¹¹ MT in the same session* (the Phase 3 debt run: target ≥ 8.0 B/s burst at 10¹⁰ = 87% of primesieve best, closing the inversion) → **G3** 6 B entries (burst headroom) → **G4** W sweep → **G5** 8S-hybrid boundary + F5 state verdict → **G6** R3 NEON tally at deep N, both clusters → **G7** A55 geometry (16 KiB option) → **R4** presieve {17,19}: run once to *record the tradeoff* — predicted single-core-positive, MT-negative on this memory-bound device (adds 0.03 B/num to save 0.13 cycles/num); expect revert, document why → **G8** checkpoint hardening + crash gauntlet → **Marathon** 10¹³ cert-record.

---

## PART 11 — THE GATE

| # | Criterion |
|---|---|
| 1 | Forced-bucket suite: enumeration ≤ 10⁷ **and** 10⁸ element-wise, pi_range invariance, N mod 30 matrix, W = 2 and W = 4 — all green |
| 2 | M-bucket, M-carry, M-ring, M-checkpoint: all killed, tiers recorded |
| 3 | Oracle full: live T3 at 10¹² exact (37,607,912,018) + randomized-5 in [10¹¹, 10¹²] vs primecount; exit 0 |
| 4 | Marathon: π(10¹³) 
# Phase 4 Gate (Part 11, Part 12, and the Gate Table)

## PART 11 — AMENDMENTS AND THE DECISION MAP

**Amendments (file_structure.md v2.3):**

1. **Tier law:** small ≤ S/4, medium (S/4, 4S], bucket (4S, √N] — boundaries are per-worker derived quantities of that worker's own S, never global constants. The 8S-hybrid (u16 tables below 330K, multiply-walk above) is documented as an experimental rung, not default.
2. **Medium-state law:** SoA constant/mutable split — per-prime constants (Δ tables, wheel rows) in shared read-only SLC-resident arrays; per-prime mutable state (offset|j) private per worker, stored absolute in unit space; u16 Δ tables.
3. **The byte-accounting law:** every design change reports Δcycles/number *and* ΔDRAM-bytes/number. A change that buys cycles with bytes is presumed guilty at multi-core until F1's measured knee says otherwise.
4. **The crash law:** any run > 5 minutes is checkpoint-resumable at unit granularity; kill-and-resume bit-exactness is a permanent gate criterion. A phone OS that can kill you at any moment is part of the spec.
5. **The certification-tier split:** live oracle certification ≤ 10¹²; 10¹³+ via cert-record protocol (constants check + field completeness + live spot-checks).
6. **The charging policy split:** correctness marathons may run on charger (flagged in the record); performance benchmarks stay unplugged. Both recorded, never confused.
7. **The forced-geometry certification pattern:** all future deep machinery (Phase 5 included) is certified at a shrunken configuration where it is fully active at exhaustively-auditable scale, before being trusted at production scale.

**Decision map — where Phase 4 outputs flow:**

| Phase 4 output | Consumer | Decision it drives |
|---|---|---|
| F1 DRAM knee curve | Every MT decision to project end | Entry-size choice (8 vs 6 B); the aggregate ceiling law at each scale |
| R2 state split (shared constants / private mutable) | Phase 5 per-worker state design | φ-tree and π-table layout discipline — the same constant/mutable separation, second consumer |
| Bucket engine + certified pi_range | Phase 5 P₂ term | Gourdon/Lehmer P₂ is literally segmented range counting — the same machinery, verbatim consumer |
| Checkpoint/resume system | titan-cli product phase | The phone-native differentiator no desktop engine has |
| Cert-record protocol | Phase 5 π(10¹⁶) marathon | How trillion-scale combinatorial runs get certified without re-running |
| Crash gauntlet harness | All future long-running machinery | The standing adversarial test for interruptibility |
| Forced-geometry certification pattern | All future machinery | The standing method for making deep-N code paths exhaustively testable |
| Per-tier telemetry counters | Phase 5 and all future benches | Per-component attribution of any model deviation |

---

## PART 12 — THE GATE

| # | Criterion |
|---|---|
| 1 | Forced-bucket suite: enumeration ≤ 10⁷ **and** 10⁸ element-wise, pi_range invariance, N mod 30 matrix, W = 2 and W = 4 — all green |
| 2 | M-bucket, M-carry, M-ring, M-checkpoint: all killed, tiers recorded |
| 3 | Oracle full: live T3 at 10¹² exact (37,607,912,018) + randomized-5 in [10¹¹, 10¹²] vs primecount; exit 0 |
| 4 | Marathon: π(10¹³) = 346,065,536,839 via cert-record (telemetry curve + checkpoint count recorded); 10¹⁴ optional badge |
| 5 | Crash gauntlet: 20 × kill -9 → resume bit-exact → final π exact |
| 6 | Zero-alloc tripwire green with bucket system live, 8 workers; sync inventory unchanged (pool only) |
| 7 | **10¹² 8T full run: ≥ 2.5 B/s sustained** (telemetry-normalized), cool burst ≥ 6.0 B/s; wall ≤ 400 s |
| 8 | Same-session primesieve 10¹² 8T head-to-head: ≥ 70% (floor), 85% stretch, delta-analysis recorded |
| 9 | Phase 3 debts closed: E3 mix table, E4 curve classified, R2/R3/R4 verdicts, 10¹⁰/10¹¹ MT re-bench post-R2 |
| 10 | F1–F6 records exist; DRAM knee reconciles with the 0.5–1.0 B/number traffic model within 2× |
| 11 | Cycle + byte ledgers at 10¹²/10¹³ reconciled within 2× via per-tier telemetry counters |
| 12 | Gate record + reference.md v4 (two-column burst/sustained at every scale; charging flags on marathon records) |

---

Run order: **F1 first** (it prices bytes in cycles for everything after), then G0 while F2/F3 cycle at forced geometry, then the ladder. Paste back F1's knee curve and G1's first 10¹² numbers — the byte ledger and the cycle ledger are both waiting to find out which one this silicon enforces first.
