Read it. The instrument finally worked — and then the report misread its own output. Phase 23's term table is the single most valuable receipt this project has produced, because it lets the **entire regression ledger close to 0.04%**. The report found one of three regressions, attached a magnitude that is arithmetically impossible, and labeled the win "root cause found." Here is the actual ground truth, then the fixes — full designs, not directives.

---

### §1. The ledger that closes

Every term below is sourced from a receipt in this session or the certified Phase 20 state (walker at 118.17 cy/cell × 776,070,926 cells = 91.7 Gcy ÷ 17.7 Gcy/s effective 8T = 5.19s; setup components 0.1013 + 0.0101 + 0.1820 are *identical* across sessions):

| Line @10¹⁴, 8T | Time | Source |
|---|---|---|
| Phase 20 true substrate | 6.015s | race R0, bit-exact |
| **R-C:** walker regression | +1.54s | K0: 118.2 → 152.9 cy/cell; components frozen at 32/30/6/18, DRAM variance 32.2 → 66.9 — *exactly* +34.7 cy/cell ≡ +1.53s |
| **R-B:** Φ_c regression | +6.64s | Φ_c = 6.015 − 5.19 − 0.41(S₂) − 0.29(setup) = 0.125s then → **6.765s** (≥54×) |
| **R-A:** gate inline Lehmer cross-check | +9.730s | 23.932 − 14.202, direct subtraction |
| **Predicted gate row** | **23.92s** | |
| **Measured (Phase 22)** | **23.932s** | residual 0.007s — **0.04%** |

Substrate-only check: 6.015 + 1.54 + 6.64 = 14.19 vs measured 14.20 ✓.

**The report found R-A — 9.73 of the 17.92 seconds — and missed R-B (6.6s) and R-C (1.5s) entirely.** 54% of the regression, presented as 100%, with the found portion mislabeled as "18.5s."

### §2. Forensic corrections

**The 18.5s cannot exist.** If the Lehmer call burned 18.5s inside a 23.932s row, substrate + Lehmer = 32.7s > 23.93s. Contradiction. The 8T cost is 9.73s by subtraction. Prediction: 18.5 ≈ 9.73 × 1.9 — that number came from timing the same call on the 2 big cores or 4T. Demand the isolating receipt; if it reads 18.5s, you measured on ≤4 effective threads.

**The 46,416 statement hides a three-way numeric collision.** At 10¹⁴: x^(1/3) = 46,416 (a *value*); π(z) = π(564,870) = 46,416 (a *count*); π(x^(1/3)) = 4,792 (what the label claims). The harness passed the z-count into a Lehmer tree whose design point was the x^(1/3)-count — a **9.7× over-deep prime base**, which is precisely why it burned ~10s. The coincidence that x^(1/3) ≈ π(2·6.085·x^(1/3)) numerically is what makes this bug class invisible to eyeballs. Rust-level kill:

```rust
#[derive(Copy, Clone)] pub struct PrimeVal(pub u64);   // p
#[derive(Copy, Clone)] pub struct PrimeIdx(pub u64);   // π(p) — array index
#[derive(Copy, Clone)] pub struct Quotient(pub u64);   // x/(p·e)
// eval_mt(x, PrimeIdx(pi_y_sqrt3)) — passing π(z) or x^(1/3) is now a compile error
```

**Still standing from Phase 22:** the K0 per-component table remains frozen constants (32.0/30.0/6.0/18.0 unchanged across sessions and code changes), with "DRAM variance" as the residual that absorbs wall-clock truth. Phase 23's *phase-level* wall times are honest; the *component* attribution is still fiction. Credit where due: K0-walker 6.7188s vs term-table D 6.7321s — two instruments agreeing to 0.2% is new and good. Also flag: PiTable "construction 0.0101s" cannot cover the v-horizon x/y ≈ 3.5×10⁸ that the walker queries — log table horizon and bytes; something is serving those lookups and it isn't this table.

**Scoreboard:** Phase 23 marked 12/12 with zero performance exit criteria while the engine sits 62× behind. Honest re-mark: ~6 PASS / 6 OWED. Phases 12 and 17 remain absent. 96.6% is theater.

### §3. The gap, decomposed as an actual proof

Replace §2.2's telemetry-reading with the model it should have been. Opponent D-hard: 0.090s × 17.7 Gcy/s ÷ ~12 cy/leaf ≈ **133M physical leaf-ops** over the 21,761-prime band (π(z) − π(y) — your "≈21,000" ✓). Titan: 776M cells at 152.9 cy = 118.7 Gcy.

**D-gap = 74.6× = surface 5.83× × per-cell 12.7×.** Two independent factors, two independent fixes. Φ_c-gap = 6.765/0.092 = **73.5×** (vs opponent Φ₀+Σ+AC complex). Setup+S₂ gap ≈ 3×. Weighted total ≈ 62× ✓ closes against the measured 62.02×. *That* is the proof — every factor owned by a mechanism.

### §4. Phase 24 — four fixes

**24-A — Harness honesty (R-A, hours).** Timing scope discipline + dial-tagged engine labels:

```rust
let t0 = now();
let pi_x = gourdon.eval_mt(x, cfg);        // substrate ONLY inside the timed row
let t_sub = now() - t0;
let leh = lehmer.eval_mt(x, PrimeIdx::of_y3(x));  // OUTSIDE timing, correct param type
assert_eq!(pi_x, leh);                     // untimed verification
```
Probe: gate 10¹⁴ row → **14.20 ± 0.25s**.

**24-B — Φ_c analytical re-wiring (R-B: 6.76s → ≤0.10s).** The recursive tree is redundant machinery for a surface Titan's tables already cover — this is the Phase 9 "Φ Collapse," certified but unwired. Flat, table-backed, magic-division:

```rust
/// Cost model @1e14: 640k iters × (4cy umulh + 30–80cy L2/L3 rank + 3cy accum)
/// ≈ 50M cy ≈ 3ms pooled. ~2000× vs the tree. Closes against the opponent's
/// own AC telemetry (0.081s = segment sieve [y,√x] + same per-prime loop).
fn phi_c_flat(x: u64, y: u64, primes: &BasePrimes, pi: &PiTable,
              md: &MagicDivSet, mert: &MertensStruct) -> i128 {
    let mut acc: i128 = 0;
    for &p in primes.iter_range(y, isqrt(x)) {          // AC-class
        acc += pi.rank(md.div(x, p)) as i128 - pi.rank(p) as i128 + 1;
    }
    acc + sigma_mu_conv(x, y, pi, mert)                  // Σ/Φ₀-class, μ-weighted
}
```
Safety against my not re-deriving your 5-term algebra from memory: **A/B term equivalence — assert `phi_c_flat == phi_c_recursive` as i128 at 10¹²/10¹³/10¹⁴, then delete the tree.** Byte budget: PiTable horizon = x/y ≈ 3.5×10⁸, ~14–16 MiB packed.

**24-C — Layout B + v-partition (R-C + the per-cell 12.7×).** Retire the global π/M lookups — the current block path *pays* (build traffic evicting table lines: DRAM variance doubled) without *collecting*. Byte-exact block:

```rust
pub const WORDS_B: usize = 512;                 // 32,768 odd residues / block
pub struct LeafBlockB {
    v_lo: u64, v_hi: u64, pi_base: u64, m_base: i32,
    odd:  [u64; 512],   // 4,096 B — 1 bit/odd residue
    mu2:  [u8; 8192],   // 8,192 B — 2 bits/odd residue (0:+1, 1:−1, 2:0)
    pi_w: [u16; 512],   // 1,024 B — π at word starts
    m_w:  [i16; 512],   // 1,024 B — M at word starts
    m_q:  [i8; 2048],   // 2,048 B — M at quarter-word starts
}                                                // TOTAL 16,416 B = 16.1 KiB ≤ L1D

#[inline] fn pi_at(&self, v: u64) -> u64 {      // ~5–7 cy
    let r = ((v - self.v_lo) >> 1) as usize; let (w, b) = (r >> 6, r & 63);
    let m = if b == 63 { !0u64 } else { (1u64 << (b+1)) - 1 };
    self.pi_base + self.pi_w[w] as u64 + (self.odd[w] & m).count_ones() as u64
}
#[inline] fn m_at(&self, v: u64) -> i32 {      // ~8–10 cy; tail ≤16 residues
    let r = ((v - self.v_lo) >> 1) as usize; let (w, b) = (r >> 6, r & 63);
    self.m_base as i32 + self.m_w[w] as i32 + self.m_q[w*4 + (b>>4)] as i32
        + self.mu2_swar_tail(w, b)              // 4-byte SWAR decode of 2-bit fields
}
```

Cost model: v-axis [√x, x/y] → ~5,200 blocks; build ≈ 200k cy/block (sieve 32,768 odds @ ~5 cy + μ-ride + prefix) ≈ 1.04 Gcy ≈ 60ms pooled ≈ **1.3 cy/cell** amortized at full surface. Parallelization: partition [√x, x/y] into census-weighted equal-cell groups; steal via `fetch_add` on a group index; per-thread private block arena, j-cursors gallop into range (O(log) × 21,761 — noise); disjoint v-ranges ⇒ disjoint cell sets ⇒ bit-exact partials by construction. NEON honesty: the *builder* vectorizes (marking + prefix); the walker's scattered `pi_at` does not — win it with ILP-2 cursor interleave and branch-free accumulation, not fictional MADD.

**24-D — z-split as a measured dial.** Hard leaves restricted to (y, z], z = β·y, everything above delegated via 24-B machinery. **Probe P24-4 first, zero implementation cost:** your census already contains the (j,v) histogram — compute `cells_hard(β)` exactly for β ∈ {1.2, 1.5, 2.0, 2.5, 3.0} and argmin `T(β) ≈ cells_hard(β)·25cy + delegate(β)` *before writing any code*. The opponent's β = 2 was tuned for its cost profile, not yours.

### §5. Projections (honest)

| @10¹⁴ 8T | now | +24-B | +24-C | +24-D | opponent |
|---|---|---|---|---|---|
| setup | 0.29 | 0.29 | 0.29 | 0.29 | ~0.02 |
| Φ_c | 6.76 | ≤0.10 | ≤0.10 | ≤0.10 | 0.092 |
| S₂ | 0.41 | 0.41 | 0.41 | 0.41 | (in AC/B) |
| D | 6.73 | 6.73 | ~1.0 | ~0.25 | 0.090 |
| **total** | **14.20** | ~7.5 | ~1.8 | **~1.05** | **0.2285** |

Phase 24 exit ≈ 1.0–1.4s = **4.5–6× behind** — not parity; parity needs Phase 25 (S₂ z-split + fused popcount sieve, setup trim, β tune) and is still not guaranteed. But the marathon window reopens: at 10¹⁸, D-hard ≈ 45–55 G cells × ~25 cy ≈ 75s 8T, S₂ becomes the capstone bottleneck (~144s → Phase 25's fusion target) → **sub-10-minute π(10¹⁸) is alive again**; it was dead at 152.9 cy/cell.

### §6. Gate

Phase 24 exit criteria: P24-1 gate row 14.20±0.25 (pre-fix); P24-2 isolated Lehmer receipt resolving 18.5 vs 9.73; P24-3 Φ_c A/B equality + ≤0.10s; P24-4 census hard-band table (before code); P24-5 Layout B 1T/8T bit-exact at 10¹²/10¹³ + cells/block + build cy/cell (E4, still owed since Phase 21); P24-6 frozen probe (10¹⁴ 8T pinned, median-of-30): **≤ 2.0s gate, 1.5s stretch**; P24-7 event-counted K0 (lookup counts × measured latency classes, reconciled to wall clock <5%) replacing the frozen constants. Re-mark Phase 23 at 6/6; re-mark 21/22 per the standing amendment; Phases 12/17 appear or get named-absorbed.

The term table did in one afternoon what a bisect would have — keep it as the permanent first-class instrument, and make every phase close against it. Next deliverable: P24-1/2/4 receipts (all measurement, no new code), then I'll write the full Layout B builder and flat Φ_c patch.
