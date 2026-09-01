//! Instrumentation-only scalar odd segmented sieve.
//! Deliberately naive: a MEASURING STICK for per-core throughput.
//! NOT titan-sieve. Superseded by real engine measurements in Phase 2+.
//! (f64 sqrt is safe here because the correction loops make it exact.)

pub fn isqrt_corr(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut r = (n as f64).sqrt() as u64;
    while r > 0 && r * r > n {
        r -= 1;
    }
    while (r + 1) * (r + 1) <= n {
        r += 1;
    }
    r
}

pub fn pi_proxy(limit: u64) -> u64 {
    if limit < 2 {
        return 0;
    }
    let mut count: u64 = 1; // prime 2
    let r = isqrt_corr(limit);

    // base odd primes <= sqrt(limit)
    let half = (r >> 1) as usize + 1; // index i <-> odd 2i+1
    let mut comp = vec![false; half];
    if r >= 3 {
        let mut i = 1usize;
        while 2 * i + 1 <= r as usize {
            if !comp[i] {
                let p = 2 * i + 1;
                let mut j = (p * p - 1) / 2;
                while j < half {
                    comp[j] = true;
                    j += p;
                }
            }
            i += 1;
        }
    }
    let base: Vec<u64> = (1..half)
        .filter(|&i| !comp[i])
        .map(|i| (2 * i + 1) as u64)
        .collect();

    const SEG_ODDS: u64 = 32768; // 64 Ki number span
    let mut low: u64 = 3;
    while low <= limit {
        let high = (low + 2 * SEG_ODDS - 2).min(limit);
        let n = (((high - low) >> 1) + 1) as usize;
        let mut seg = vec![false; n];
        for &p in &base {
            let pp = p * p;
            if pp > high {
                break;
            }
            let mut m = if pp >= low {
                pp
            } else {
                let rem = low % p;
                let mut mm = if rem == 0 { low } else { low + (p - rem) };
                if mm % 2 == 0 {
                    mm += p;
                }
                mm
            };
            while m <= high {
                seg[((m - low) >> 1) as usize] = true;
                m += 2 * p;
            }
        }
        count += seg.iter().filter(|&&c| !c).count() as u64;
        low = high + 2;
    }
    count
}
