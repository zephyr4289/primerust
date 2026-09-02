//! Phase 30: 32-bit Packed Factor Table (Ftd) with Log-Wavefront Parallel Build.
//!
//! Entry layout: [lpf_idx: 14 | sign: 1 | nz: 1 | mpf: 16]
//! - lpf_idx (bits 0..14): prime index of smallest prime factor, or SENTINEL (0x3FFF) if prime
//! - sign (bit 14): 1 if mu(n) == -1, 0 if mu(n) == +1
//! - nz (bit 15): 1 if mu(n) == 0 (square non-zero kill bit), 0 if squarefree
//! - mpf (bits 16..31): largest prime factor, clipped to 65535

pub struct Ftd {
    pub e: Vec<u32>, // z + 1 entries, 4 B/entry (40 MiB at z = 10^7)
}

pub const LPF: u32 = 0x3FFF;
pub const SENT: u32 = 0x3FFF; // prime sentinel: composites' lpf_idx <= pi(sqrt(z)) <= 454 << 16383
pub const SGN: u32 = 1 << 14;
pub const NZB: u32 = 1 << 15;

impl Ftd {
    #[inline(always)]
    pub fn nz(&self, n: u64) -> bool {
        self.e[n as usize] & NZB != 0
    }

    #[inline(always)]
    pub fn sign(&self, n: u64) -> u32 {
        (self.e[n as usize] >> 14) & 1
    }

    #[inline(always)]
    pub fn mu(&self, n: u64) -> i32 {
        if self.nz(n) {
            0
        } else if self.sign(n) == 1 {
            -1
        } else {
            1
        }
    }

    #[inline(always)]
    pub fn mpf(&self, n: u64) -> u32 {
        self.e[n as usize] >> 16
    }

    #[inline(always)]
    pub fn lpf_idx(&self, n: u64) -> u32 {
        self.e[n as usize] & LPF
    }

    #[inline(always)]
    pub fn is_prime(&self, n: u64) -> bool {
        self.lpf_idx(n) == SENT
    }

    /// primes: ascending list <= sqrt(z) (index i = pi(p) among these).
    pub fn build(z: u64, primes: &[u32]) -> Ftd {
        assert!(primes.len() < SENT as usize, "lpf 14-bit ceiling: z < 3.3e10");
        let mut e = vec![SENT; z as usize + 1];

        // 1 is squarefree, mu(1) = +1, mpf = 1
        e[1] = SENT | (1 << 16);

        // Pass 0: Initialize primes <= sqrt(z)
        for &p in primes {
            if (p as u64) <= z {
                e[p as usize] = SENT | SGN | ((p.min(0xFFFF) as u32) << 16);
            }
        }

        // Pass 1: Write-only strided stores for lpf (descending prime order -> smallest prime wins)
        for (i, &p) in primes.iter().enumerate().rev() {
            let (p_u64, mut m) = (p as u64, (p as u64) * (p as u64));
            while m <= z {
                e[m as usize] = i as u32;
                m += p_u64;
            }
        }

        // Pass 2: Sequential / log-wavefront resolution of mu and mpf
        for nn in 2..=z as usize {
            let li = (e[nn] & LPF) as usize;
            if li == SENT as usize {
                if (e[nn] & SGN) == 0 {
                    // Primes in (sqrt(z), z]
                    e[nn] = SENT | SGN | ((nn.min(0xFFFF) as u32) << 16);
                }
                continue;
            }
            let p = primes[li] as usize;
            let q = nn / p; // Dependent load

            let eq = e[q];
            let lq = (eq & LPF) as usize;

            // Lemma 5 (D1 fix): mu(n) = 0 <==> mu(q) = 0 || p | q
            let p_div_q = if lq == SENT as usize {
                q == p
            } else {
                lq == li
            };
            let nz = (p_div_q || (eq & NZB != 0)) as u32;
            let sign = ((eq & SGN) ^ SGN) >> 14; // Flip sign: mu(n) = -mu(q)

            // mpf inherits from q (or p if q == 1, but q >= 2 for composites)
            let mpf_val = eq & 0xFFFF_0000;

            e[nn] = (li as u32) | (sign << 14) | (nz << 15) | mpf_val;
        }

        Ftd { e }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_lpf_and_mu(n: u64) -> (u64, i32, u64) {
        if n <= 1 {
            return (0, 1, 1);
        }
        let mut temp = n;
        let mut lpf = 0u64;
        let mut mpf = 0u64;
        let mut num_prime_factors = 0;
        let mut d = 2u64;
        while d * d <= temp {
            if temp % d == 0 {
                if lpf == 0 {
                    lpf = d;
                }
                mpf = d;
                num_prime_factors += 1;
                temp /= d;
                if temp % d == 0 {
                    // Square factor detected
                    while temp % d == 0 {
                        temp /= d;
                    }
                    // Find remaining mpf
                    let mut r = d + 1;
                    while r * r <= temp {
                        if temp % r == 0 {
                            mpf = r;
                            while temp % r == 0 {
                                temp /= r;
                            }
                        }
                        r += 1;
                    }
                    if temp > 1 {
                        mpf = temp;
                    }
                    return (lpf, 0, mpf);
                }
            }
            d += 1;
        }
        if temp > 1 {
            if lpf == 0 {
                lpf = temp;
            }
            mpf = temp;
            num_prime_factors += 1;
        }
        let mu = if num_prime_factors % 2 == 0 { 1 } else { -1 };
        (lpf, mu, mpf)
    }

    #[test]
    fn test_ftd_d1_defect_n18() {
        let base_primes = titan_sieve::base::generate_base_primes(100);
        let primes_u32: Vec<u32> = base_primes.iter().map(|&p| p as u32).collect();

        let ft = Ftd::build(100, &primes_u32);

        // n = 18 = 2 * 3^2 -> mu(18) MUST be 0 (D1 defect check)
        assert_eq!(ft.mu(18), 0, "D1 defect check failed: mu(18) must be 0!");
        assert!(ft.nz(18));
        assert_eq!(ft.mu(12), 0);
        assert_eq!(ft.mu(27), 0);
        assert_eq!(ft.mu(6), 1); // 2 * 3 -> mu = +1
        assert_eq!(ft.mu(30), -1); // 2 * 3 * 5 -> mu = -1
    }

    #[test]
    fn test_ftd_comprehensive_against_naive() {
        let z = 10_000u64;
        let base_primes = titan_sieve::base::generate_base_primes(titan_core::roots::isqrt(z) + 10);
        let primes_u32: Vec<u32> = base_primes.iter().map(|&p| p as u32).collect();

        let ft = Ftd::build(z, &primes_u32);

        for n in 2..=z {
            let (_, naive_mu, naive_mpf) = naive_lpf_and_mu(n);
            let ft_mu = ft.mu(n);
            assert_eq!(
                ft_mu, naive_mu,
                "mu({}) mismatch! ft={}, naive={}",
                n, ft_mu, naive_mu
            );
            if naive_mu != 0 {
                assert_eq!(
                    ft.mpf(n) as u64, naive_mpf.min(0xFFFF),
                    "mpf({}) mismatch! ft={}, naive={}",
                    n, ft.mpf(n), naive_mpf
                );
            }
        }
    }
}
