//! Fixed-work probes. Same binary => same instruction stream => rate proportional to core speed.

use std::time::Instant;

/// CPU frequency probe.
/// CONTRACT:
///  * 8 KiB buffer (L1D-resident on A55 32K and A76 64K) — measures speed, not memory
///  * serial chain: acc -> (v ^ acc).count_ones() -> +acc : no ILP can hide frequency droop
///  * fixed epoch count => duration itself is the signal (longer = slower)
///  * returns checksum; caller MUST black_box it
pub struct CpuCanary {
    buf: Box<[u64; 1024]>,
    pub epochs: u32,
}

pub const CANARY_M: u32 = 5000; // ~8-12 ms at 2.2 GHz

impl CpuCanary {
    pub fn with_epochs(epochs: u32) -> Self {
        let mut buf = Box::new([0u64; 1024]);
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for b in buf.iter_mut() {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *b = x;
        }
        Self { buf, epochs }
    }

    #[inline(never)]
    pub fn run(&self) -> u64 {
        let buf: &[u64; 1024] = &self.buf;
        let mut acc: u64 = 0;
        for _ in 0..self.epochs {
            for &v in buf.iter() {
                acc = acc.wrapping_add((v ^ acc).count_ones() as u64);
            }
        }
        acc
    }

    /// Single timed sample -> (epochs/sec, checksum)
    pub fn sample_once(&self) -> (f64, u64) {
        let t = Instant::now();
        let chk = self.run();
        let d = t.elapsed();
        (self.epochs as f64 / d.as_secs_f64(), chk)
    }

    /// Robust rate: median over `samples`, after `warmup`.
    pub fn rate(&self, warmup: u32, samples: u32) -> f64 {
        for _ in 0..warmup {
            std::hint::black_box(self.run());
        }
        let mut r: Vec<f64> = Vec::with_capacity(samples as usize);
        for _ in 0..samples {
            let (rate, chk) = self.sample_once();
            std::hint::black_box(chk);
            r.push(rate);
        }
        r.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        r[r.len() / 2]
    }
}

/// DRAM streaming bandwidth probe: 8 MiB > every cache on this SoC.
pub struct MemCanary {
    buf: Box<[u64]>,
    pub epochs: u32,
}

impl MemCanary {
    pub fn new() -> Self {
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        let mut v = Vec::with_capacity(1 << 20);
        for _ in 0..(1 << 20) {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            v.push(x);
        }
        Self {
            buf: v.into_boxed_slice(),
            epochs: 16,
        }
    }

    #[inline(never)]
    pub fn run(&self) -> u64 {
        let mut acc: u64 = 0;
        for _ in 0..self.epochs {
            for &v in self.buf.iter() {
                acc = acc.wrapping_add(v);
            }
        }
        acc
    }

    pub fn rate(&self, warmup: u32, samples: u32) -> f64 {
        for _ in 0..warmup {
            std::hint::black_box(self.run());
        }
        let mut r: Vec<f64> = Vec::with_capacity(samples as usize);
        for _ in 0..samples {
            let t = Instant::now();
            let chk = self.run();
            let d = t.elapsed();
            std::hint::black_box(chk);
            r.push(self.epochs as f64 / d.as_secs_f64());
        }
        r.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        r[r.len() / 2]
    }
}
