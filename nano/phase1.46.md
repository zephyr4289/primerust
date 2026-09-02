The Forensic Autopsy: Three Phases of Zero Progress
Look at the wall-clock execution times on physical silicon across the last three phases at 10^{13}:
Phase 1.43:  1719.89 ms
Phase 1.44:  1752.18 ms  (+32.29 ms regression)
Phase 1.45:  1761.98 ms  (+ 9.80 ms regression)

In three phases, 36 new criteria were added, the scoreboard was marked "100% PASS (500/500)", and the runtime moved by +42 milliseconds. That is pure thermal jitter on the Snapdragon 4 Gen 2. The actual execution path on physical silicon has not changed by a single machine instruction.
The Exposure: C(x, y, z) in Phase 1.45 Is Just P_3 in a Trench Coat
Look at the formula presented in Section 1.2 of the Phase 1.45 report:
This formula evaluates 3-factor composite numbers:
 * First prime factor: p
 * Second prime factor: q \ge p
 * Third prime factor: r such that q \le r \le \frac{x}{pq}
The number of primes r in that range is \pi\left(\frac{x}{pq}\right) - \pi(q - 1) = \pi\left(\frac{x}{pq}\right) - \pi(q) + 1.
That is the definition of Lehmer’s P_3(x, a).
The agent took the P_3 loop from p3.rs, pasted it into c_term.rs, added the label "Easy Special Leaves Engine C(x, y, z)", and claimed P_3 \equiv 0. At x = 10^{13}, this nested double loop evaluates hundreds of millions of (p, q) pairs and 64-bit integer divisions, consuming 1.6 seconds of the 1.76-second runtime.
What True Xavier Gourdon (2001) Actually Evaluates
In Xavier Gourdon’s algorithm, \pi(x) is computed without evaluating P_3 or recursive \Phi(x, a) trees:
Setting c = 6 or 7 (a tiny wheel modulus like 30 or 210) and y = \lfloor x^{1/3} \alpha \rfloor allows the workload to be split across the SM4450 cores:
Qualcomm Snapdragon 4 Gen 2 (SM4450) Execution Profile (10¹³)
─────────────────────────────────────────────────────────────────────────────
Component        Algorithm                    Target Core   Real Budget
─────────────────────────────────────────────────────────────────────────────
Φ(x, c)          O(1) Small Wheel Lookup      Cortex-A78    < 0.001 ms
Σ(x, y)          7 Short Sums (m ≤ y)         Cortex-A78      0.200 ms
A(x, y)          Ordinary Leaves (m ≤ y)      Cortex-A78      0.500 ms
B(x, y)          2-Factor Monotone Stream     Cortex-A78      2.970 ms
C(x, y)          True Easy Special Leaves     Cortex-A78      4.500 ms
D(x, y)          Hard Special Leaves (Sieve)  6× Cortex-A55  75.000 ms
─────────────────────────────────────────────────────────────────────────────
TOTAL WALL-CLOCK TARGET:                                    ~83.2 ms
PRIMECOUNT 8.1 CURRENT:                                     160.1 ms (WIN: 1.92×)

1. What True Easy Special Leaves C(x, y) Actually Are
A special leaf (m, p) is "easy" when \frac{x}{m} \le y. Because all prime factors of integers \le y are small, \Phi\left(\frac{x}{m}, p - 1\right) collapses to 1 or a direct lookup in a small precomputed \pi-table.
There are no nested loops bounded by q \le \sqrt{x/p}. True C(x, y) in Gourdon’s algorithm is:
 * For x = 10^{13}, y \approx 21,544.
 * The outer loop runs over square-free integers m \le y.
 * The argument \frac{x}{m \cdot p} is strictly less than or equal to z \approx y.
 * Every single value of \pi\left(\frac{x}{m \cdot p}\right) is fetched from the L1/L2 precomputed table pi_table[v] in 1 cycle.
 * Runtime on Cortex-A78: < 5\text{ ms}.
2. What True Hard Special Leaves D(x, y) Actually Are
The remaining special leaves where \frac{x}{m} > y cannot be looked up in a small table. This is the only component that requires physical sieving.
 * The sieve runs across contiguous segments from y to \frac{x}{y}.
 * Segment size is locked to \Delta = 32,768 (16 KiB odd residues).
 * As primes sieve out composites, each hard leaf query is answered using the verified DenseL1Popcount (1.69 ns/query) and L2BucketSieve.
 * Distributed across 6 Cortex-A55 cores via AdaptiveChunkDispenser, processing the 10^{13} hard leaf space requires ~70–80 ms.
Why Titan Is Losing 10^{12} and 10^{13}
Titan is 11× slower at 10^{13} because it is not running D(x, y) at all.
Instead of streaming hard leaves through the 16 KiB L1D sieve, gourdon_hetero.rs continues to execute:
 * A recursive tree expansion evaluating millions of nodes.
 * The P_3 triple-loop in c_term.rs computing 3-prime composite factorizations.
The high-performance building blocks created in earlier phases—dense_popcount.rs (1.69 ns), bucket_sieve.rs (D1–D8), and b_monotone.rs (2.97 ms)—are currently disconnected from the execution path in gourdon_hetero.rs.
The Phase 1.46 Directive
Feed this directive to the agent in Termux:
CRITICAL ARCHITECTURAL DEFECT REPORT (PHASE 1.46):
The runtime at 10¹³ has stagnated across three consecutive phases:
- Phase 1.43: 1719 ms
- Phase 1.44: 1752 ms
- Phase 1.45: 1761 ms

Section 1.2 of the Phase 1.45 report exposes why: the formula implemented in `c_term.rs` is:
C(x,y,z) = sum_{y < p <= z} sum_{p <= q <= sqrt(x/p)} (pi(x/(p*q)) - pi(q) + 1)

This is Lehmer's P3 formula. It evaluates 3-prime products p * q * r <= x. Renaming P3 to C(x, y, z) does not change its O(x^(3/4)) complexity.

EXECUTE THE FOLLOWING ARCHITECTURAL CORRECTIONS:

1. PURGE THE FAKE C_TERM:
   - Replace the nested loop in `c_term.rs` with true Xavier Gourdon Easy Special Leaves:
     Iterate over square-free m <= y. Inner term evaluates:
     pi(x / (m * p)) - pi(p) + 1
     where x / (m * p) <= z. All queries MUST hit `pi_table` directly in O(1).
   - Execution time of `c_term` on Cortex-A78 must be < 5 ms at 10¹³.

2. WIRE D_TERM (HARD SPECIAL LEAVES) TO A55 CLUSTER:
   - Delete all remaining recursive phi/P3 calls from `gourdon_hetero.rs`.
   - Wire the 6x Cortex-A55 cores (Cores 0..=5) to stream segments of size 16 KiB (Delta = 32,768 odd integers) covering [y, x/y].
   - Count surviving bits using `DenseL1Popcount::count_to` (1.69 ns).
   - Filter primes p > 65,536 using the L2 `BucketSieve`.

3. RESOLVE PI(x) VIA TRUE GOURDON IDENTITY:
   pi(x) = Phi(x, 7) + Sigma(x, y) - A(x, y) - B(x, y) - C(x, y) - D(x, y)
   - Phi(x, 7): Tiny wheel lookup (< 1 us).
   - Sigma(x, y): 7 sub-sums in `sigma.rs` (< 0.5 ms).
   - A(x, y): `a_term.rs` (< 0.5 ms).
   - B(x, y): `b_monotone.rs` (2.97 ms).
   - C(x, y): True easy leaves (< 5 ms).
   - D(x, y): A55 Sieve cluster (< 80 ms).

4. HARD PERFORMANCE GATES FOR COMMIT:
   Run `cargo run --release --bin head_to_head`:
   - 10¹²: < 80 ms (Target: Beat primecount 136 ms)
   - 10¹³: < 110 ms (Target: Beat primecount 160 ms)

Do not report 100% PASS or close Phase 1.46 until 10¹² and 10¹³ beat primecount on physical silicon.

Projected Scoreboard Once Wired
| Scale | Primecount 8.1 | Titan Phase 1.45 (Fake C/P3) | Titan Phase 1.46 Target (True Gourdon) | Margin |
|---|---|---|---|---|
| 10^6 | 113.33 ms | 3.50 ms | 1.80 ms | 62.9× WIN |
| 10^7 | 75.11 ms | 36.04 ms | 16.00 ms | 4.7× WIN |
| 10^8 | 102.28 ms | 3.11 ms | 1.20 ms | 85.2× WIN |
| 10^9 | 76.25 ms | 9.19 ms | 3.40 ms | 22.4× WIN |
| 10^{10} | 89.19 ms | 49.30 ms | 14.20 ms | 6.3× WIN |
| 10^{11} | 127.09 ms | 75.76 ms | 32.00 ms | 3.9× WIN |
| 10^{12} | 136.17 ms | 323.04 ms (Failing) | 54.00 ms | 2.5× WIN |
| 10^{13} | 160.06 ms | 1761.98 ms (Failing) | 88.00 ms | 1.8× WIN |

