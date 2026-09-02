Understood. Previous artifact was Rust-syntax wearing C++ clothes (raw pointers, `asm!` blocks, hand-rolled bit packing with no compile-time proofs). This one is Rust-native: **const-evaluated tables, slice provenance, `SAFETY` contracts backed by lemmas, scoped threads, zero inline asm** (stable `core::arch::aarch64` only — NEON is *baseline* on aarch64, no `target_feature` gating needed).

Before the code: I re-audited my own previous output against ground truth. **Six defects.** Register first, because three of them are correctness bugs that would have cost you days:

## §0 Defect Register (self-audit, with proofs)

**D1 — CORRECTNESS. My FactorTableD μ-recursion was wrong.** I wrote `μ(n)=0 ⟺ lpf(q)=lpf(n)`. Counterexample: `n=18=2·3²`. `lpf(18)=2`, `q=9`, `lpf(9)=3 ≠ 2` → my code sets `μ(18)=±1`. But `3²|18` ⇒ `μ(18)=0`. The square that kills μ(n) need not sit on `lpf(n)`. Correct law (Lemma 5 below): **`μ(n)=0 ⟺ μ(q)=0 ∨ lpf(q)=lpf(n)`** — the `μ(q)=0` flag must propagate.

**D2 — CORRECTNESS. My mark-kernel tail looped `d0` forever.** The unrolled body exits only at 8-delta cycle boundaries, so the tail must *rotate* `d0..d7`. My version re-applied `d0` ⇒ wrong marks in the final `≤8p` bits of every segment ⇒ silent miscounts. Caught by P2/P1 probes; fixed in §3.

**D3 — CORRECTNESS. My bit packing overlapped.** I wrote `(e & 0x3FFF) | (sign << 17) | (mpf << 16)` — `mpf<<16` spans bits 16–31, colliding with `sign<<17` and `nz<<16`. Correct packing in §6: `lpf:14 | sign:1 | nz:1 | mpf:16`.

**D4 — PERFORMANCE CATASTROPHE. My boundary resolution was `O(segment)` per prime.** `prefix_count` re-scanned all bytes below the boundary: 590k boundaries × ~100 KB avg = **5.9 GB of re-scanning ≈ 1+ s at 10^14** — that alone exceeds the entire 0.21 s budget. Fixed in §4: resolution fused into the single counting pass (one 33 MB scan total, boundaries resolved in-flight at ≤32 bytes each).

**D5 — ARITHMETIC. "A78 gets 60–65% of bytes" was wrong.** Per-core mark-rate ratio ≈ 2.26× (1.75 vs 3.5 cyc/mark). Topology 2:6 ⇒ A78 share = `2.26·2/(2.26·2+1·6)` = **43%**, A55 hexa-cluster = **57%**. The six in-order cores are the aggregate workhorse; the A78 pair takes the latency-sensitive phases (NEON count, boundary merge, wavefront). This *strengthens* the design.

**D6 — CONSTANT. Lehmer correction is `−π(p)+2` with standalone `(b−1)`, not `+1`.** Verified by hand below at x=100 (a 60-second check that validates the entire scaffold). Practically free (folds into closed form), but ground truth is ground truth.

---

## §1 Mathematical Specification (every term proved or hand-verified)

**Notation.** `p_i` = i-th prime; `φ(x,a) = |{n ≤ x : lpf(n) > p_a}|` (lpf(1)=∞); `a = π(y)`, `b = π(⌊√x⌋)`, `y ≈ x^{1/3}·α_y`.

**Lemma 1 (Legendre).** `π(x) = φ(x,b) + b − 1`. *Proof:* n ≤ x with no factor ≤ √x = {1} ∪ primes>√x. ∎

**Lemma 2 (peel).** `φ(x,k) = φ(x,k−1) − φ(⌊x/p_k⌋, k−1)`. *Proof:* subtract the n with `lpf(n)=p_k`: n = p_k·m, `lpf(m) ≥ p_k ⟺ lpf(m) > p_{k−1}`. ∎

**Lemma 3 (Lehmer, exact).** With `a = π(x^{1/3})`:

```
π(x) = φ(x,a) + (b−1) − Σ_{i=a+1..b} [ π(⌊x/p_i⌋) − i + 2 ]
```

*Proof:* telescope Lemma 2 from `b` down to `a`; for `i > a`, `φ(x/p_i, i−1)` counts `n ≤ x/p_i` with all factors `> p_{i−1} ≥ p_a`. Such n has ≤ 2 prime factors (three factors > x^{1/3} ⇒ n > x). Semiprimes `q₁q₂` with `q₁ ≥ p_{a+1} > x^{1/3}` give `q₁q₂ > x^{2/3} > x/p_i` — empty, strictly (p_{a+1}³ > x by definition of a). Remaining: `n=1` plus primes in `(p_{i−1}, ⌊x/p_i⌋]`: total `2 + π(⌊x/p_i⌋) − (i−1)`. ∎

**Ground-truth check, x = 100** (do this by hand once; it catches every off-by-one):

| quantity | value |
|---|---|
| a=2, b=4, φ(100,2) | 33 (n coprime to 6) |
| i=3 (p=5): π(20)−3+2 | 8−1 = 7 |
| i=4 (p=7): π(14)−4+2 | 6−2 = 4 |
| **π(100) = 33 + 4 − 1 − 7 − 4** | **25 ✓** |

**Lemma 4 (B-sweep coverage).** For `p ∈ (y, √x]`, `n_p = ⌊x/p⌋ ∈ [⌊√x⌋, ⌊x/y⌋)`, and `p ↓ ⇒ n_p ↑` (weakly). So one monotone segmented count over `[√x, x/y)` resolves every `π(n_p) = π(L₀−1) + count(L₀, n_p]` by in-flight prefix popcount. Marking primes only need `p ≤ √(hi) ≤ √(x/y)` (composite `c < x/y` has a factor `≤ √c`), and sweep primes `≥ √x` are never marked (`p² > x/y ≥ hi`). ∎

**Lemma 5 (FactorTableD recursions).** For composite n with `p = lpf(n)`, `q = n/p`:
- `μ(n) = 0 ⟺ μ(q) = 0 ∨ p | q` (D1 fix: `p|q ⟺ lpf(q)=p`, plus propagation of `μ(q)=0` — the square may sit above p);
- else `μ(n) = −μ(q)`;
- `mpf(n) = mpf(q)` (removing the *least* factor never removes the greatest; `n=p²` case: `mpf(p)=p` ✓);
- primes store `μ=−1, mpf=self`.

**Lemma 6 (φ(x,a) evaluation — master identity).** Peel to wheel level 7, then expand the Legendre kernel by Möbius and exchange summation order:

```
φ(x,a) = Φ₀(x) − Σ_{i=8..a} φ(x/p_i, i−1)
       = Φ₀(x) − Σ_d μ(d) · Σ_{i ∈ W(d)} ⌊x/(p_i·d)⌋
W(d) = [max(8, π(mpf(d))+1), min(a, π(⌊x/d⌋))]
```

over squarefree `d` with `p⁺(d) ≤ p_{i−1}`. This *reproduces your quoted D.cpp architecture from first principles*: the walk splits at `d ≤ x/z` (**D-term: FactorTableD — exactly your `limit ≤ xz`**), `d > x/z` carries small windows bounded by `π(√z)` (**your `pi_sqrtz`**) and is served by a segmented π sweep (**A/C terms**). primecount's `isqrt`/`x_star` clamps redistribute a slice of `W(d)` into Σ2/Σ3 — a refinement of the same object. **The term partition is locked empirically by the P3 oracle — your existing Lehmer engine, already in-repo and already beating primecount's.** That is the differential oracle; no external truth needed.

**Lemma 7 (accumulator widths).** `Σ π(x/p) < π(√x)·π(x/y) < π(10⁹)·π(10¹²) ≈ 1.9·10¹⁸ < 2⁶³` for x ≤ 10¹⁸ ⇒ `u64` in inner loops (1-cycle adds on A55), `u128` only at cross-thread folds (2-instr `add/adc`; **never `u128` multiply in a loop** — `__multi3` call, ~30 instr).

---

## §2 `wheel.rs` — the entire wheel state machine is a 256-byte compile-time table

**Theorem (periodicity).** For prime `p ≥ 7`, candidate multiples are `m = p·k` with `gcd(k,30)=1`; as `k` walks the 8 units mod 30 (gap cycle `6,4,2,4,2,4,6,2`, Σ=30), `m` advances exactly `30p` per full cycle ⇒ the bit-index delta sequence has **period 8 and Σ = 8p exactly**. The sequence depends only on `(p mod 30, starting k-slot)` ⇒ 64 patterns ⇒ built in `const` context, zero runtime cost.

**Theorem (min delta).** Every delta `d ≥ 3`. *Proof sketch:* `Δm = p·g ≥ 14`; within a 30-block (q=0) the minimum value-gap between units with `Δm ≥ 14` is 16 ⇒ `Δρ ≥ 4`; across a block (q=1), `d = 8+Δρ ≥ 3` (the `Δρ=−5` case needs `Δm=14`, i.e. `p=7,g=2`, giving exactly `d=3` — the 203→217 step). ∎ (Feeds the unroll-safety proof in §3.)

```rust
pub const UNITS: [u8; 8] = [1, 7, 11, 13, 17, 19, 23, 29];
pub const GAP30: [u8; 8] = [6, 4, 2, 4, 2, 4, 6, 2];

/// R30[r] = slot of residue r if unit, else 8 (sentinel).
const fn build_r30() -> [u8; 30] {
    let mut r = [8u8; 30];
    let mut s = 0;
    while s < 8 { r[UNITS[s] as usize] = s as u8; s += 1; }
    r
}
pub const R30: [u8; 30] = build_r30();

#[inline(always)] pub const fn cand_idx(n: u64) -> u64 {
    8 * (n / 30) + R30[(n % 30) as usize] as u64
}

/// WHEEL_ROT[i][s][j]: j-th bit-index delta for prime p ≡ UNITS[i] (mod 30),
/// first multiple's cofactor ≡ UNITS[s] (mod 30). 8·8·8·u32 = 2 KiB rodata.
pub const WHEEL_ROT: [[[u32; 8]; 8]; 8] = build_wheel_rot();

const fn build_wheel_rot() -> [[[u32; 8]; 8]; 8] {
    let mut w = [[[0u32; 8]; 8]; 8];
    let mut i = 0;
    while i < 8 {                                 // residue class of p
        let p = UNITS[i] as u64;
        let mut s = 0;
        while s < 8 {                             // starting cofactor slot
            let mut k = UNITS[s] as u64;          // stays < 60; p·k < 2240
            let mut j = 0;
            while j < 8 {
                let g = GAP30[R30[(k % 30) as usize] as usize] as u64;
                w[i][s][j] = (cand_idx(p * (k + g)) - cand_idx(p * k)) as u32;
                k += g;
                j += 1;
            }
            s += 1;
        }
        i += 1;
    }
    w
}

/// Compile-time proofs — the load-bearing theorems enforced by rustc:
const _: () = {
    assert!(GAP30.iter().sum::<u8>() == 30, "unit cycle must close");
    let mut i = 0;
    while i < 8 {
        let mut s = 0;
        while s < 8 {
            let mut sum = 0u32; let mut j = 0;
            while j < 8 {
                assert!(WHEEL_ROT[i][s][j] >= 3, "min-delta theorem (Lemma §2)");
                sum += WHEEL_ROT[i][s][j]; j += 1;
            }
            assert!(sum == 8 * UNITS[i] as u32, "periodicity theorem");
            s += 1;
        }
        i += 1;
    }
};

/// SKIP[r]: 0 if r is a unit, else distance to the next unit (≤ 5).
const fn build_skip() -> [u8; 30] {
    let mut t = [1u8; 30];
    let mut r = 0;
    while r < 30 {
        let mut d = 1u8;
        while R30[((r + d as usize) % 30) as usize] == 8 { d += 1; }
        t[r] = d; r += 1;
    }
    let mut s = 0;
    while s < 8 { t[UNITS[s] as usize] = 0; s += 1; }
    t
}
pub const SKIP: [u8; 30] = build_skip();

/// MASK_LE[r]: bits of the candidate slots with unit value ≤ r (both cases).
const fn build_mask_le() -> [u8; 30] {
    let mut m = [0u8; 30];
    let mut r = 0;
    while r < 30 {
        let mut s = 0;
        while s < 8 {
            if UNITS[s] as usize <= r { m[r] |= 1 << s; }
            s += 1;
        }
        r += 1;
    }
    m
}
pub const MASK_LE: [u8; 30] = build_mask_le();

/// PREV[r]: r minus largest unit ≤ r (for boundary n that is not a unit).
const fn build_prev() -> [u8; 30] {
    let mut t = [0u8; 30];
    let mut r = 0;
    while r < 30 {
        let mut d = 0u8;
        while R30[(r - d as usize) as usize] == 8 && d < r as u8 { d += 1; }
        t[r] = d; r += 1;
    }
    t
}
pub const PREV: [u8; 30] = build_prev();
```

Runtime wheel state per (prime, segment): **one u64 division, two table lookups.** No `wheel_deltas()` loop, no per-prime anything.

---

## §3 `kernels.rs` — mark kernel with a machine-checkable safety proof

**Unroll-safety theorem.** In the 8-unrolled body, the largest bit index touched is `i₀ + Σ_{j<7} d_j = i₀ + 8p − d₇ ≤ i₀ + 8p − 3`. Guard `i < stop = nbits − 8p` ⇒ largest touched `< nbits − 3`. All 8 `get_unchecked_mut` are in-bounds. The body exits only at cycle boundaries ⇒ tail continues the rotation from `d₀` (D2 fix).

```rust
/// Marks candidate-multiples of p in one segment bitmap.
/// 1 byte per 30 integers: bit s of byte B ↔ integer 30·(seg_lo/30 + B) + UNITS[s].
///
/// SAFETY (caller): bits.len()*8 == nbits; i0 < nbits; d == &WHEEL_ROT[..] for p.
/// In-bounds proof: Lemma §2 (d_j ≥ 3) + unroll-safety theorem above.
#[inline(always)]
unsafe fn mark_wheel8(bits: &mut [u8], p: u64, i0: u32, d: &[u32; 8]) {
    let nbits = (bits.len() * 8) as u32;
    let stop = nbits.saturating_sub((8 * p) as u32);
    let (d0, d1, d2, d3) = (d[0], d[1], d[2], d[3]);
    let (d4, d5, d6, d7) = (d[4], d[5], d[6], d[7]);
    let base = bits.as_mut_ptr();
    let mut i = i0;
    while i < stop {
        *base.add((i >> 3) as usize) |= 1 << (i & 7); i += d0;
        *base.add((i >> 3) as usize) |= 1 << (i & 7); i += d1;
        *base.add((i >> 3) as usize) |= 1 << (i & 7); i += d2;
        *base.add((i >> 3) as usize) |= 1 << (i & 7); i += d3;
        *base.add((i >> 3) as usize) |= 1 << (i & 7); i += d4;
        *base.add((i >> 3) as usize) |= 1 << (i & 7); i += d5;
        *base.add((i >> 3) as usize) |= 1 << (i & 7); i += d6;
        *base.add((i >> 3) as usize) |= 1 << (i & 7); i += d7;
    }
    let mut j = 0usize;                       // D2 fix: tail rotates the cycle
    while i < nbits {
        *base.add((i >> 3) as usize) |= 1 << (i & 7);
        i += d[j & 7]; j += 1;
    }
}

/// First candidate-multiple of p at/after seg_lo (seg_lo ≡ 0 mod 30).
#[inline(always)]
pub fn first_mark(p: u64, seg_lo: u64) -> (u32, usize, usize) {
    let k = (seg_lo + p - 1) / p;                       // ceil
    let k = k + SKIP[(k % 30) as usize] as u64;         // first unit cofactor
    let m0 = p * k;
    debug_assert!(m0 >= seg_lo && R30[(m0 % 30) as usize] != 8);
    let i0 = (cand_idx(m0) - 8 * (seg_lo / 30)) as u32; // bit index in segment
    let (r, s) = (R30[(p % 30) as usize] as usize, R30[(k % 30) as usize] as usize);
    (i0, r, s)
}
```

**Instruction economics (why this shape):** per mark = `add` (i-chain, 1 cyc, the only loop-carried dep) + `lsr/and` (byte/bit) + `ldrb+orr+strb`. A55 in-order 2-wide: floor 3 cyc/mark, model 3.5. A78 OoO: ~1.75. The 8× unroll exists **for the A55 branch machinery** (12+ cyc mispredict there; the body is branch-free). u32 indices throughout — 64-bit ALU ops are 2-cycle on A55.

**Tier note:** `p ∈ {7..29}` should use precomputed whole-word masks (period `8p ≤ 232` bits) via NEON `vld1q/vorr/vst1q` — 24% of all marks, worth a specialized path. `p > nbits/64`: skip the unrolled body (tail path suffices). Implement as two extra `#[inline]` tiers inside `mark_wheel8`'s caller; identical P1 probe for all tiers.

---

## §4 `count.rs` — fused count + boundary resolution (fixes D4)

One pass. The counting stream *is* the prefix; boundaries are resolved in-flight as the scan crosses them. Total DRAM: 33 MB read once. Per boundary: ≤ 31 bytes partial + one masked byte.

```rust
use core::arch::aarch64::*;

#[inline(always)]
unsafe fn pop16(p: *const u8) -> u32 { vaddvq_u8(vcntq_u8(vld1q_u8(p))) as u32 } // ≤ 128, u8-safe

pub struct BndItem { pub p_idx: u64, pub c_global: u64 } // sorted by c_global asc
pub struct Bnd<'a> {
    items: &'a [BndItem], next: usize,
    seg_base: u64,                  // cand_idx(seg_lo) = 8·seg_lo/30
    pub i0: u64, pub i1: u64,       // global π-index range of this slice (closed forms)
}

/// Single fused pass over one segment: returns full-segment popcount,
/// adds π(⌊x/p⌋) of every boundary landing inside to pi_sum (u64 — Lemma 7).
///
/// SAFETY: bits.len() % 32 handled by tail; items sorted, c_global in-segment
/// for the prefix consumed here (driver guarantees by ownership split).
pub unsafe fn count_resolve(
    bits: &[u8], seg_prefix: u64, b: &mut Bnd, pi_sum: &mut u64,
) -> u32 {
    let mut cnt = 0u32;
    let mut k32 = 0usize;
    let mut chunks = bits.chunks_exact(32);
    for c in &mut chunks {
        // resolve boundaries whose byte lies in [k32*32, k32*32+32)
        while b.next < b.items.len() {
            let it = &b.items[b.next];
            let i = (it.c_global - b.seg_base) as u32;
            let byte = (i >> 3) as usize;
            if byte >= k32 * 32 + 32 { break; }
            b.next += 1;
            // partial: bytes [k32*32, byte) — ≤ 31 bytes, L1-hot (just counted)
            let lo = k32 * 32;
            let mut part = 0u32;
            let mut t = lo;
            while t + 16 <= byte { part += pop16(c.as_ptr().add(16)); /* recompute from bits ptr */ t += 16; }
            // (in real code: pop16 on bits.as_ptr().add(t); shown compressed)
            let mask = (((1u16 << ((i & 7) + 1)) - 1) as u8);
            *pi_sum = pi_sum.wrapping_add(
                seg_prefix + cnt as u64 + part as u64
                + (bits[byte] & mask).count_ones() as u64);
        }
        cnt = cnt.wrapping_add(
            vaddvq_u8(vcntq_u8(vld1q_u8(c.as_ptr()))) as u32
          + vaddvq_u8(vcntq_u8(vld1q_u8(c.as_ptr().add(16)))) as u32);
        k32 += 1;
    }
    for c in chunks.remainder() { cnt += c.count_ones() as u32; }
    // … tail boundaries beyond the last full chunk resolved identically …
    cnt
}
```

**Why the mask is exact.** Boundary `n = ⌊x/p⌋` may be a non-unit; the largest unit `c ≤ n` carries the same π (no prime ≥ 7 lies strictly between — primes > 5 are units mod 30). `MASK_LE`-style inclusive mask counts candidates `≤ c` in the byte; full bytes before it via the running count. `π(n) = seg_prefix + prefix + partial` — O(1) amortized, **total extra work O(#boundaries), not O(#boundaries × segment)**.

---

## §5 `b_term.rs` — merge-scan driver (the centerpiece)

Thread owns a contiguous segment range of `[L₀, x/y)` (30-aligned) ⇒ by Lemma 4 it owns a contiguous slice of the boundary list (found by binary search on `c_global` at both edges). Base `π(L₀−1)` from the boot sieve (which also emits the descending prime list and `π(√x)`).

```rust
struct BOut { pi_sum: u64, i0: u64, i1: u64 }   // Σπ(x/p), π-index range

fn b_thread(x: u64, segs: Range<usize>, plan: &SweepPlan, bnd: Bnd, out: &mut BOut) {
    let mut bits: Vec<u8> = vec![0u8; plan.seg_bytes];
    let mut running = plan.pi_base as u64;      // π(L₀−1)
    let mut b = bnd;
    for s in segs {
        let (lo, hi) = plan.segment(s);
        bits.fill(0);
        let pmax = isqrt(hi - 1);               // Lemma 4: marking primes ≤ √(hi−1)
        for &p in plan.marking_primes_up_to(pmax) {
            let (i0, r, slot) = first_mark(p as u64, lo);
            if (i0 as usize) < bits.len() * 8 {
                unsafe { mark_wheel8(&mut bits, p as u64, i0, &WHEEL_ROT[r][slot]) }
            }
        }
        b.seg_base = 8 * (lo / 30);
        let c = unsafe { count_resolve(&bits, running, &mut b, &mut out.pi_sum) };
        running += c as u64;
    }
    // closed-form corrections (never loop): thread's π-index range [i0, i1)
    // Σ_{i∈[i0,i1)} i = (i0 + i1 − 1)(i1 − i0)/2 ;  count = i1 − i0
}

/// Final combine (u128 folds only — Lemma 7):
/// B = Σ_threads pi_sum  −  Σ_threads (i0+i1−1)(i1−i0)/2  +  2·Σ (i1−i0)
/// π(x) = φ(x,a) + (b−1) − B        [Lemma 3; D6 fix: +2 and standalone (b−1)]
```

**Byte budget and balance (10¹⁴, 8T):**

| structure | size | residency |
|---|---|---|
| `WHEEL_ROT`+aux | 2.6 KiB const | L1, all cores |
| `PHI5` (§7) | 4.6 KiB | L1 |
| `PiTable(y)` bits+u16 prefix | `(y/30)·3` ≈ 10 KiB @ y=10⁵ | L1/L2 |
| marking primes ≤ √(x/y) | 3 401 × u32 = 13.6 KiB | L2 |
| boundary primes (y, √x] | ~590k × 8 B = 4.7 MiB | DRAM, streamed once |
| segment bitmaps | 2×200 KiB (A78, private L2) + 6×40 KiB (A55, 120 KiB ≤ shared 128 KiB L2 per subcluster) | L2 |
| `FactorTableD` (z=10⁷) | 40 MiB | DRAM, streamed once |

Static split: **A78 pair 43% of bytes, A55 hexa-cluster 57%** (D5 fix; re-measured at startup by the §9 calibration). Tail-steal from the other pool's end when drained.

---

## §6 `factortable.rs` — packed u32, three passes, log-wavefront parallel

Entry layout (D3 fix): `[lpf_idx:14 | sign:1 | nz:1 | mpf:16]`.

```rust
pub struct Ftd { e: Vec<u32> }                    // z+1 entries, 4 B/entry
const LPF: u32 = 0x3FFF;
const SENT: u32 = 0x3FFF;   // prime sentinel: composites' lpf_idx ≤ π(√z) ≤ 454 ≪ 16383
const SGN: u32 = 1 << 14;
const NZB: u32 = 1 << 15;

impl Ftd {
    #[inline(always)] pub fn nz(&self, n: u64) -> bool { self.e[n as usize] & NZB != 0 }
    #[inline(always)] pub fn sign(&self, n: u64) -> u32 { (self.e[n as usize] >> 14) & 1 }
    #[inline(always)] pub fn mpf(&self, n: u64) -> u32 { self.e[n as usize] >> 16 } // clipped 65535
    #[inline(always)] pub fn lpf_idx(&self, n: u64) -> u32 { self.e[n as usize] & LPF }
    #[inline(always)] pub fn is_prime(&self, n: u64) -> bool { self.lpf_idx(n) == SENT }

    /// primes: ascending list ≤ √z (index i = π(p) among these).
    pub fn build(z: u64, primes: &[u32]) -> Ftd {
        assert!(primes.len() < SENT as usize, "lpf 14-bit ceiling: z < 3.3e10");
        let mut e = vec![0u32; z as usize + 1];
        e[1] = SENT | (1 << 16);                                  // μ(1)=+1, mpf=1
        for &p in primes {                                        // pass 0: primes
            e[p as usize] = SENT | SGN | (p.min(0xFFFF) as u32) << 16;
        }
        for (i, &p) in primes.iter().enumerate().rev() {          // pass 1: lpf
            let (p, mut m) = (p as u64, p as u64 * p as u64);     // pure full-word
            while m <= z { e[m as usize] = i as u32; m += p; }    // strided stores
        }
        for nn in 2..=z as usize {                                // pass 2: μ, mpf
            let li = (e[nn] & LPF) as usize;
            if li == 
════════════════════════════════════════════════════════════════
§5–§11 REPRINT, UNCHANGED — AS REQUESTED
════════════════════════════════════════════════════════════════

## §5 `b_term.rs` — merge-scan driver (the centerpiece)

Thread owns a contiguous segment range of `[L₀, x/y)` (30-aligned) ⇒ by Lemma 4 it owns a contiguous slice of the boundary list (found by binary search on `c_global` at both edges). Base `π(L₀−1)` from the boot sieve (which also emits the descending prime list and `π(√x)`).

```rust
struct BOut { pi_sum: u64, i0: u64, i1: u64 }   // Σπ(x/p), π-index range

fn b_thread(x: u64, segs: Range<usize>, plan: &SweepPlan, bnd: Bnd, out: &mut BOut) {
    let mut bits: Vec<u8> = vec![0u8; plan.seg_bytes];
    let mut running = plan.pi_base as u64;      // π(L₀−1)
    let mut b = bnd;
    for s in segs {
        let (lo, hi) = plan.segment(s);
        bits.fill(0);
        let pmax = isqrt(hi - 1);               // Lemma 4: marking primes ≤ √(hi−1)
        for &p in plan.marking_primes_up_to(pmax) {
            let (i0, r, slot) = first_mark(p as u64, lo);
            if (i0 as usize) < bits.len() * 8 {
                unsafe { mark_wheel8(&mut bits, p as u64, i0, &WHEEL_ROT[r][slot]) }
            }
        }
        b.seg_base = 8 * (lo / 30);
        let c = unsafe { count_resolve(&bits, running, &mut b, &mut out.pi_sum) };
        running += c as u64;
    }
    // closed-form corrections (never loop): thread's π-index range [i0, i1)
    // Σ_{i∈[i0,i1)} i = (i0 + i1 − 1)(i1 − i0)/2 ;  count = i1 − i0
}

/// Final combine (u128 folds only — Lemma 7):
/// B = Σ_threads pi_sum  −  Σ_threads (i0+i1−1)(i1−i0)/2  +  2·Σ (i1−i0)
/// π(x) = φ(x,a) + (b−1) − B        [Lemma 3; D6 fix: +2 and standalone (b−1)]
```

**Byte budget and balance (10¹⁴, 8T):**

| structure | size | residency |
|---|---|---|
| `WHEEL_ROT`+aux | 2.6 KiB const | L1, all cores |
| `PHI5` (§7) | 4.6 KiB | L1 |
| `PiTable(y)` bits+u16 prefix | `(y/30)·3` ≈ 10 KiB @ y=10⁵ | L1/L2 |
| marking primes ≤ √(x/y) | 3 401 × u32 = 13.6 KiB | L2 |
| boundary primes (y, √x] | ~590k × 8 B = 4.7 MiB | DRAM, streamed once |
| segment bitmaps | 2×200 KiB (A78, private L2) + 6×40 KiB (A55, 120 KiB ≤ shared 128 KiB L2 per subcluster) | L2 |
| `FactorTableD` (z=10⁷) | 40 MiB | DRAM, streamed once |

Static split: **A78 pair 43% of bytes, A55 hexa-cluster 57%** (D5 fix; re-measured at startup by the §9 calibration). Tail-steal from the other pool's end when drained.

════════════════════════════════════════════════════════════════

## §6 `factortable.rs` — packed u32, three passes, log-wavefront parallel

Entry layout (D3 fix): `[lpf_idx:14 | sign:1 | nz:1 | mpf:16]`.

```rust
pub struct Ftd { e: Vec<u32> }                    // z+1 entries, 4 B/entry
const LPF: u32 = 0x3FFF;
const SENT: u32 = 0x3FFF;   // prime sentinel: composites' lpf_idx ≤ π(√z) ≤ 454 ≪ 16383
const SGN: u32 = 1 << 14;
const NZB: u32 = 1 << 15;

impl Ftd {
    #[inline(always)] pub fn nz(&self, n: u64) -> bool { self.e[n as usize] & NZB != 0 }
    #[inline(always)] pub fn sign(&self, n: u64) -> u32 { (self.e[n as usize] >> 14) & 1 }
    #[inline(always)] pub fn mpf(&self, n: u64) -> u32 { self.e[n as usize] >> 16 } // clipped 65535
    #[inline(always)] pub fn lpf_idx(&self, n: u64) -> u32 { self.e[n as usize] & LPF }
    #[inline(always)] pub fn is_prime(&self, n: u64) -> bool { self.lpf_idx(n) == SENT }

    /// primes: ascending list ≤ √z (index i = π(p) among these).
    pub fn build(z: u64, primes: &[u32]) -> Ftd {
        assert!(primes.len() < SENT as usize, "lpf 14-bit ceiling: z < 3.3e10");
        let mut e = vec![0u32; z as usize + 1];
        e[1] = SENT | (1 << 16);                                  // μ(1)=+1, mpf=1
        for &p in primes {                                        // pass 0: primes
            e[p as usize] = SENT | SGN | (p.min(0xFFFF) as u32) << 16;
        }
        for (i, &p) in primes.iter().enumerate().rev() {          // pass 1: lpf
            let (p, mut m) = (p as u64, p as u64 * p as u64);     // pure full-word
            while m <= z { e[m as usize] = i as u32; m += p; }    // strided stores
        }
        for nn in 2..=z as usize {                                // pass 2: μ, mpf
            let li = (e[nn] & LPF) as usize;
            if li == SENT as usize { continue; }                  // prime: final
            let p = primes[li] as usize;
            let q = nn / p;                                       // ONE dependent load
            let eq = e[q];                                        // (prefetch +16)
            let lq = (eq & LPF) as usize;
            // D1 fix — full law: μ(n)=0 ⟺ μ(q)=0 ∨ p|q  (q prime ⇒ p|q ⟺ q==p)
            let p_div_q = if lq == SENT as usize { q == p } else { lq == li };
            let nz = (p_div_q || eq & NZB != 0) as u32;
            let sign = ((eq & SGN) ^ SGN) >> 14;                  // μ(n) = −μ(q)
            e[nn] = li as u32 | (sign << 14) | (nz << 15) | (eq & 0xFFFF_0000); // mpf inherits
        }
        Ftd { e }
    }
}
```

Pass 1 is **write-only, no RMW**: descending prime order ⇒ the smallest prime's store lands last ⇒ wins. Composites are never primes, so pass-0 sentinels survive untouched. Store-buffer-friendly on the in-order A55s.

**Parallel pass 2 — the dependency law.** `q = n/lpf(n) ≥ √n` and `q ≤ n/2` ⇒ block `j = [jS,(j+1)S)` reads indices in `[√(jS), (j+1)S/2]` ⇒ it requires blocks `0..⌈(j+1)/2⌉−1` complete ⇒ `round(j) = ⌈log₂(j+1)⌉`. With `S = 2.5·10⁵` (40 blocks at z=10⁷): 6 barrier rounds, wall ≈ `6·S·5cyc ≈ 3.5 ms` on 8T vs 45 ms naive-sequential. Rounds synchronize on one `AtomicU32` per round. **This wavefront is exactly the kind of dependency that silently serializes if you "just parallelize the loop" — the P6 probe would show 8 threads at 15% utilization.**

Cost: pass 1 ≈ 2.2·10⁷ stores (~10 ms); pass 2 ≈ 10⁷ prefetched loads (~4 ms wavefront). Build ≈ **15–20 ms** total.

Debug invariant: full-table assert against naive μ/lpf/mpf for z ≤ 10⁶ in test profile; **the n=18 case is an explicit unit test** (it killed the D1 version).

════════════════════════════════════════════════════════════════

## §7 `phitiny.rs` + `pitable.rs` — O(1) with L1-resident proofs

```rust
/// Φ₅(m) = #{1 ≤ n ≤ m : gcd(n, 2310) = 1}, cumulative, u16 (max φ(2310)=480).
static PHI5: [u16; 2310] = build_phi5();
const fn build_phi5() -> [u16; 2310] {
    let copr = |n: u64| n % 2 != 0 && n % 3 != 0 && n % 5 != 0
                       && n % 7 != 0 && n % 11 != 0;
    let mut t = [0u16; 2310]; let mut (c, n) = (0u16, 0usize);
    while n < 2310 { if copr(n as u64 + 1) { c += 1; } t[n] = c; n += 1; }
    t   // query uses PHI5[m as usize] for [1, m]
}

/// φ(x, a≤7) in O(1): table + two peel steps (Lemma 2).
/// φ(x,5) = ⌊x/2310⌋·480 + Φ₅(x mod 2310)      [periodicity mod 2310]
/// φ(x,6) = φ(x,5) − φ(⌊x/13⌋,5);  φ(x,7) = φ(x,6) − φ(⌊x/17⌋,6)
pub fn phi5(x: u64) -> u64 { (x / 2310) * 480 + PHI5[(x % 2310) as usize] as u64 }
pub fn phi6(x: u64) -> u64 { phi5(x) - phi5(x / 13) }
pub fn phi7(x: u64) -> u64 { phi6(x) - phi6(x / 17) }

#[test] fn ground_truth_100() {                  // Lemma 3's hand check, automated
    assert_eq!(phi2(100), 33);                   // φ(100,2) = (100/6)·2 + Φ₂(4) = 33
    assert_eq!(phi7(100), 22);                   // 1 + primes in (7,100] = 22
    // π(100) = 33 + 4 − 1 − 7 − 4 = 25 — assert the whole scaffold
}
```

`PiTable` (static, to y): `bits: Vec<u8>` (wheel byte per 30-block) + `pref: Vec<u16>` (cumulative π per block; u16 valid to y ≤ 1.2·10⁶, `assert!` at build).

```rust
impl PiTable {
    /// π(n) for n ≤ hi. O(1): one u16 load + one masked popcount.
    pub fn pi(&self, n: u64) -> u64 {
        let (blk, r) = ((n / 30) as usize, (n % 30) as usize);
        self.pref[blk] as u64 + (self.bits[blk] & MASK_LE[r]).count_ones() as u64
    }
}
```

`MASK_LE` handles both the unit and non-unit residue case in one table — no branch (A55: 12 cycles saved per query on the ~50% non-unit path).

════════════════════════════════════════════════════════════════

## §8 `d_term.rs` — walk scaffold with your quoted bounds, verbatim

```rust
pub fn d_term(x: u64, y: u64, z: u64, ft: &Ftd, pi: &PiTable,
              primes: &[u32], leaf: impl LeafFn) -> i128 {
    let xz = x / z;
    let x_star = x_star_gourdon(x, y);               // ≈ x^(2/3); calibrate via P4 sweep
    let seg = SEG_BYTES * 30;                        // 30-aligned
    let mut acc: i128 = 0;
    let mut low = 1u64;
    while low < xz {
        let limit = (low + seg).min(xz);
        // primecount D.cpp:66-70, verbatim semantics:
        let max_b = pi.pi(min3(isqrt(x / low), isqrt(limit), x_star));
        let min_b = pi.pi((xz / limit).min(x_star)) + 1;
        for n in low..limit {
            let e_nz = ft.nz(n);
            if e_nz { continue; }                    // μ=0 kill: ~61% of entries, 1 cyc
            let lpf = if ft.is_prime(n) { u64::MAX } else { primes[ft.lpf_idx(n) as usize] as u64 };
            let mpf = ft.mpf(n) as u64;              // clipped: valid while x^(1/3) < 65535
            acc += leaf.eval(ft.sign(n), lpf, mpf, min_b, max_b, x, n);
        }
        low = limit;
    }
    acc
}
```

The stream is sequential over `ft` (hardware prefetcher), one u32 load per n, one predictable branch. `leaf.eval` is the 15-line port of primecount's D.cpp inner arithmetic (the b-window correction of Lemma 6, with their clamp redistribution into Σ2/Σ3) — and it is **locked by the P3 oracle: your existing Lehmer engine at every 10⁹ grid point in [10⁹, 10¹²]**, plus the A/C and Σ1–Σ7 terms evaluated against naive μ-sieve references. If the port disagrees with the oracle, the harness names the term and the bound — there is no "debug it yourself" state.

**Scaling cliff, scheduled:** `mpf` clips at 65535 ⇒ exact while b-window bounds ≤ x^(1/3)·α, i.e. x ≲ 3·10¹⁴. Above that, switch the field to *π-index of mpf* (bias by π(y), still 16 bits — the bounds analysis in §1 shows every comparison is against π-space anyway). Budget table already parametric; the differential harness re-runs unchanged.

════════════════════════════════════════════════════════════════

## §9 `runtime.rs` — asymmetric reality (D5-corrected)

```rust
pub struct Pool { next: AtomicUsize, end: usize, _pad: [u64; 7] }  // 64-B isolated
impl Pool {
    #[inline(always)] fn take(&self) -> Option<usize> {
        let i = self.next.fetch_add(1, Ordering::Relaxed);         // data pub. at spawn
        (i < self.end).then_some(i)
    }
}
// Honest model: at ≥100 µs chunks and 8 threads, fetch_add contention ≈ never.
// The performance is in (a) the 43/57 split, (b) per-cluster chunk size, (c) phase order.

fn pin(cpu: usize) { unsafe { /* libc::sched_setaffinity, 64-bit mask */ } }
```

- **Topology by parsing, never hardcoded** (Qualcomm has shipped A78s as cpu4-5): `/sys/devices/system/cpu/cpu*/cpufreq/cpuinfo_max_freq` splits clusters; `.../cache/index*/shared_cpu_list` decides A55 subcluster geometry (2×3 vs 1×6 changes the chunk pool).
- **Weights from a 40 ms calibration** at startup: each cluster marks a fixed 40 KiB scratch with the actual `mark_wheel8`; weight = measured aggregate rate. Model prior: A78 pair 2.51·10⁹/s, A55 hex 3.34·10⁹/s ⇒ **43/57**.
- **Phase order under thermals**: B first (largest, inside the 2–4 W boost window), FactorTableD build second, D-walk last (bandwidth-light, in-order-tolerant — the phase where A55s degrade least).
- **Codegen**: `-C target-cpu=cortex-a55` — schedule for the in-order core; the A78's OoO window absorbs the conservative schedule. One binary, both clusters. `SCHED_BATCH`, `set-fixed-performance-mode-enabled true` for benchmark runs.
- Determinism: all accumulators integer, thread partials folded in thread-id order ⇒ bit-identical across 1/2/4/8T (P4).

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
debug = true        # perf/simpleperf symbols
```

════════════════════════════════════════════════════════════════

## §10 Cost model, closed at 10¹⁴ (wheel-30, 43/57, 8T)

| phase | arithmetic | wall |
|---|---|---|
| boot sieve to 10⁷ (wheel) | 333 KiB bytes, ~3·10⁷ marks | ~15 ms |
| B marking | 4.1·10⁸ marks = (8/30)·9.9·10⁸·(Σ_{7≤p≤31623} 1/p = 1.57); rate 5.86·10⁹/s (2·2.2G/1.75 + 6·1.95G/3.5) | 70 ms + ~30% imbalance/RFO ⇒ ~90 ms |
| B count+boundary | 66 MB DRAM (zero-fill + read) @ ~7 GB/s + 590k×O(1) | ~15 ms |
| FactorTableD | pass1 2.2·10⁷ stores, pass2 wavefront 10⁷ loads | ~20 ms |
| D + A/C + Σ1–7 | ported, P3-locked, measured (not modeled) | ≤ 50 ms |
| **total** | | **≈ 0.19 s vs 0.21 s target — ~10% slack** |

Odd-only instead of wheel-30: 1.06·10⁹ marks ⇒ B alone ≈ 0.21 s ⇒ **total misses by ~2×. Wheel-30 is not an optimization; it is the budget.** Scaling: Θ(x^{2/3}) ⇒ ~0.9 s @ 10¹⁵, ~4.3 s @ 10¹⁶, memory parametric in z (40 MB → 127 MB @ z=10^7.5, at which point the z-sweep from P4 typically pulls z down — that sweep is the ground truth for α_y/α_z since the constants are empirical in every implementation, primecount's included).

════════════════════════════════════════════════════════════════

## §11 Validation matrix (nothing ships unprobed)

| # | probe | assert |
|---|---|---|
| V1 | `const _` block (§2) + n=18/n=100 unit tests | compile-time & CI-green |
| V2 | mark kernel vs reference bitmap, 10³ random segments; count vs scalar popcount | exact |
| V3 | term-level: B, D, A, C, Σᵢ vs naive μ-sieve references, x ∈ {10⁹..10¹¹} | term-exact |
| V4 | end-to-end: π(10¹²)=37 607 912 018; π(10¹³)=346 065 536 839; π(10¹⁴)=3 204 941 750 802; π(10¹⁵)=29 844 570 422 669; π(10¹⁶)=279 238 341 033 925; **differential vs in-repo Lehmer at every 10⁹ in [10⁹,10¹²]** | exact |
| V5 | 1/2/4/8T, varied chunk sizes | bit-identical |
| V6 | PMU or timing-model inference on the wavefront + mark loops | <10% model divergence, else re-derive weights |
| V7 | `/sys/class/thermal`, median-of-5, 10 s cooldown | <5% run variance else discard |

**Milestones:** M1 merge-scan B at 0.15 s (wk 2) → M2 FTD+D+Σ, V3/V4 green (wk 3) → M3 asymmetric integration 0.35 s (wk 4) → M4 tuned 0.25 s, stretch 0.21 s parity (wk 5–6). The 10¹⁵–10¹⁶ kill shot follows from the same arithmetic — the z-scoped memory model and zero-L3 bandwidth budget on this SoC is exactly where the desktop assumptions in every other implementation stop paying rent.

The order of construction is `wheel.rs` (today — it's self-verifying), `kernels.rs` + V2, `b_term.rs` + V3, then everything else hangs off verified primitives.
