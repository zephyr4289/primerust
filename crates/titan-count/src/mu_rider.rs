//! Segmented Mu-Rider (R3a Deliverable).
//!
//! Rides the physical segmented sieve loop to compute mu(d) across
//! the extended domain [x^(1/2), x^(2/3)].
//!
//! Uses:
//!   - p^2-marking for squarefree detection (only sieving primes p <= x^(1/3))
//!   - omega-parity XOR bit-flips on each prime crossing (sign = (-1)^omega)

pub struct MuSegmentRider {
    pub seg_size: usize,
    pub is_squarefree: Vec<bool>,
    pub omega_parity: Vec<u8>, // 0 for even omega (+1), 1 for odd omega (-1)
}

impl MuSegmentRider {
    pub fn new(seg_size: usize) -> Self {
        Self {
            seg_size,
            is_squarefree: vec![true; seg_size],
            omega_parity: vec![0; seg_size],
        }
    }

    /// Resets rider bit-planes for a new segment window [lo, hi]
    #[inline(always)]
    pub fn reset(&mut self) {
        self.is_squarefree.fill(true);
        self.omega_parity.fill(0);
    }

    /// Marks prime crossing: flips parity and marks p^2 multiples
    #[inline(always)]
    pub fn mark_prime(&mut self, p: u64, seg_lo: u64, seg_hi: u64) {
        let span = (seg_hi - seg_lo + 1) as usize;
        let p_sq = p * p;

        // 1. Flip omega parity on prime multiples
        let start_p = if seg_lo % p == 0 {
            seg_lo
        } else {
            seg_lo + (p - seg_lo % p)
        };
        let mut idx_p = (start_p - seg_lo) as usize;
        while idx_p < span {
            self.omega_parity[idx_p] ^= 1;
            idx_p += p as usize;
        }

        // 2. Mark p^2 square multiples
        if p_sq <= seg_hi {
            let start_sq = if seg_lo % p_sq == 0 {
                seg_lo
            } else {
                seg_lo + (p_sq - seg_lo % p_sq)
            };
            let mut idx_sq = (start_sq - seg_lo) as usize;
            while idx_sq < span {
                self.is_squarefree[idx_sq] = false;
                idx_sq += p_sq as usize;
            }
        }
    }

    /// Evaluates mu(d) for index within segment
    #[inline(always)]
    pub fn mu_at(&self, idx: usize) -> i8 {
        if !self.is_squarefree[idx] {
            0
        } else if self.omega_parity[idx] == 0 {
            1
        } else {
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mu_rider_basic() {
        let mut rider = MuSegmentRider::new(100);
        rider.reset();

        let primes = [2u64, 3, 5, 7];
        for &p in &primes {
            rider.mark_prime(p, 1, 100);
        }

        // Check small values
        // 1 is squarefree, omega=0 => mu(1) = 1
        assert_eq!(rider.mu_at(0), 1); // offset 0 corresponds to 1
        // 2 is prime, omega=1 => mu(2) = -1
        assert_eq!(rider.mu_at(1), -1);
        // 3 is prime, omega=1 => mu(3) = -1
        assert_eq!(rider.mu_at(2), -1);
        // 4 = 2^2 => mu(4) = 0
        assert_eq!(rider.mu_at(3), 0);
        // 6 = 2*3 => omega=2, squarefree => mu(6) = 1
        assert_eq!(rider.mu_at(5), 1);
    }
}
