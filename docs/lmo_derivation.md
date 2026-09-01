# Project Titan: LMO & Gourdon Combinatorial Prime Counting Derivation (D-Lock)

```
Document:   docs/lmo_derivation.md
Status:     FROZEN & D-LOCKED (Phase 7 Deliverable)
Authority:  Derived from Lagarias-Miller-Odlyzko (1987), Oliveira e Silva (2006), and Xavier Gourdon (2001)
Target:     titan-count v2 Combinatorial Engine
```

---

## 1. Executive Summary & The D-Lock Law

This document establishes the exact mathematical identities, symbol conventions, summation limits, sign rules, and algorithmic invariants for the **Lagarias–Miller–Odlyzko (LMO)** and **Xavier Gourdon** prime counting engines implemented in `titan-count`.

Under the **D-Lock Law**, all formulas in this codebase are derived and numerically anchored before implementation. No formula or sign convention is transcribed without verification.

---

## 2. The Meissel Identity ($P_3$ Vanishing Foundation)

### 2.1 Theorem & Mathematical Statement
Let $x \ge 1$, $a = \pi(y)$ with $p_a^3 > x$ (so $y \ge x^{1/3}$), and $b = \pi(\lfloor \sqrt{x} \rfloor)$. Then:

$$\pi(x) = \Phi(x, a) + a - 1 - S_2(x, a, b)$$

where:
$$S_2(x, a, b) = \sum_{i=a+1}^b \left[ \pi\left(\left\lfloor \frac{x}{p_i} \right\rfloor\right) - i + 1 \right]$$

### 2.2 Proof / Derivation
1. Let $\mathcal{S} = \{ n \in [1, x] : \gcd(n, p_1 p_2 \dots p_a) = 1 \}$. By definition, $|\mathcal{S}| = \Phi(x, a)$.
2. Any composite number $n \in \mathcal{S}$ must have all prime factors $> p_a$.
3. Since $p_a > x^{1/3}$, any composite with 3 or more prime factors must satisfy $n \ge p_{a+1}^3 > x$, which is impossible for $n \le x$.
4. Therefore, every composite $n \in \mathcal{S}$ has **exactly two prime factors**: $n = p \cdot q$ with $p_a < p \le q \le x/p$.
5. The elements of $\mathcal{S}$ consist of:
   - The integer $1$ (1 element).
   - Primes $p \le p_a$ do NOT belong to $\mathcal{S}$ (except none, they are sifted out).
   - Primes $p \in (p_a, x]$ (there are $\pi(x) - a$ such primes).
   - Semiprimes $p \cdot q$ with $p_a < p \le q$ and $p \cdot q \le x$.
6. Thus:
   $$\Phi(x, a) = 1 + (\pi(x) - a) + S_2(x, a, b)$$
7. Rearranging for $\pi(x)$:
   $$\pi(x) = \Phi(x, a) + a - 1 - S_2(x, a, b) \quad \blacksquare$$

### 2.3 Relation to $P_2$
Notice that:
$$S_2(x, a, b) = \sum_{i=a+1}^b \pi\left(\left\lfloor \frac{x}{p_i} \right\rfloor\right) - \sum_{i=a+1}^b (i - 1) = P_2(x, a, b) - \frac{(b + a - 1)(b - a)}{2}$$

Therefore:
$$\pi(x) = \Phi(x, a) + a - 1 - P_2(x, a, b) + \frac{(b + a - 1)(b - a)}{2}$$

### 2.4 Worked Numerical Anchors
* **Anchor 1: $x = 100$**:
  - $x^{1/3} \approx 4.64 \implies a = \pi(5) = 3$ ($p_3 = 5, 5^3 = 125 > 100$).
  - $x^{1/2} = 10 \implies b = \pi(10) = 4$ ($p_4 = 7$).
  - $\Phi(100, 3) = \Phi(100, 2 \cdot 3 \cdot 5) = 26$.
  - $S_2(100, 3, 4) = \sum_{i=4}^4 [\pi(\lfloor 100/7 \rfloor) - 4 + 1] = \pi(14) - 3 = 6 - 3 = 3$.
  - $\pi(100) = 26 + 3 - 1 - 3 = \mathbf{25}$ (Exact!).

* **Anchor 2: $x = 1,000$**:
  - $a = \pi(11) = 5$ ($p_5 = 11, 11^3 = 1331 > 1000$).
  - $b = \pi(31) = 11$.
  - $\Phi(1000, 5) = 207$.
  - $S_2(1000, 5, 11) = 43$.
  - $\pi(1000) = 207 + 5 - 1 - 43 = \mathbf{168}$ (Exact!).

---

## 3. LMO Combinatorial Leaf Decomposition

In Lagarias–Miller–Odlyzko (1987), the totient term $\Phi(x, a)$ is decomposed into ordinary leaves and special leaves:

$$\Phi(x, a) = S_0 + S_1 + S_2'$$

### 3.1 Ordinary Leaves ($S_0$)
Leaves where the divisor $d$ has no factors beyond small primes or evaluates directly in the $\Phi_{\text{tiny}}$ base case:
$$S_0 = \sum_{d \mid P_k} \mu(d) \left\lfloor \frac{x}{d} \right\rfloor = \Phi(x, k) = \left\lfloor \frac{x}{P_k} \right\rfloor \phi(P_k) + \Phi(x \bmod P_k, k)$$

### 3.2 Special Leaves ($S_1$ and $S_2'$)
Special leaves occur when a prime $p_j$ branches in the recursion tree $\Phi(y, j) = \Phi(y, j-1) - \Phi(\lfloor y/p_j \rfloor, j-1)$:
- An ordinary leaf terminates when $\lfloor y/p_j \rfloor < p_j$ (i.e. $\lfloor y/p_j \rfloor < p_j$, yielding $\Phi(\lfloor y/p_j \rfloor, j-1) = 1$).
- A special leaf $d = m \cdot p_j$ terminates when $p_j \le \lfloor y/p_j \rfloor < p_j^2$, yielding $\pi(\lfloor y/p_j \rfloor) - j + 1$.

---

## 4. Architecture Theorem & Proof

### Theorem 4.1 (Leaf Lookup Span Bound)
For $a = \pi(x^{1/3})$, every multi-factor leaf $d = p_j \cdot e$ ($e > 1$) satisfies:
$$\left\lfloor \frac{x}{d} \right\rfloor < x^{1/2}$$

**Proof**:
Let $d = p_j \cdot e$ with $e \ge p_{j+1}$.
- **Case 1**: $p_j \le x^{1/4}$.
  The leaf termination condition requires $x/d < p_j^2$. Since $p_j \le x^{1/4}$, $x/d < (x^{1/4})^2 = x^{1/2}$.
- **Case 2**: $p_j > x^{1/4}$.
  Since $e \ge p_{j+1} > p_j > x^{1/4}$, we have $d = p_j \cdot e > x^{1/4} \cdot x^{1/4} = x^{1/2}$.
  Therefore, $\lfloor x/d \rfloor \le x / d < x / x^{1/2} = x^{1/2}$. $\blacksquare$

### Corollary 4.2 (The RAM Law Preservation)
1. A prefix $\pi$-table of span $\mathbf{x^{1/2}}$ suffices to evaluate 100% of multi-factor leaf queries in $O(1)$ time.
2. Single-factor terms ($d = p_i$) and $S_2$ threshold terms lie exclusively in $[x^{1/2}, x/p_{a+1}) \subseteq [x^{1/2}, x^{2/3})$.
3. Therefore, a single physical sieve sweep over $[x^{1/2}, x^{2/3})$ with an intra-segment walk epilogue serves all remaining terms in a single pass (**The One-Pass Law**).

---

## 5. Xavier Gourdon (2001) Refinement: 5-Term Identity

Gourdon optimizes the summation by partitioning:

$$\pi(x) = AC(x, y) - B(x, y) + D(x, y) + \Phi_0(x, k) + \Sigma(x, y)$$

where:
1. **$\Phi_0(x, k)$**: Closed-form totient via `titan-core::phi_tiny` ($k \le 6$ flat in L1D).
2. **$\Sigma(x, y)$**: Small prime products sum, evaluated via binary search over primes $\le x^{1/3}$.
3. **$AC(x, y)$**: Ordinary leaves and easy combinatorial leaves evaluated in $O(1)$ via the $\pi$-table.
4. **$B(x, y)$ & $D(x, y)$**: Hard special leaves accumulated in parallel over the single-pass segmented range sweep $[x^{1/2}, x^{2/3})$.
