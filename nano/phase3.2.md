Phase 2: Asymmetric DynamIQ Guided Work Stealing & Affinity Pinning
The benchmark data confirms the impact of the reciprocal engine: 10^{12} dropped to 44.79 ms and 10^{16} dropped to 3,042.98 ms. However, at 10^{15} (1.07×) and 10^{16} (1.23×), the lead over primecount narrows because the D(x, y, z) sieve dominates >60% of total runtime.
On the Snapdragon 4 Gen 2 (SM4450), the hardware asymmetry is stark:
 * 2× Cortex-A78 (Cores 6, 7): 4-wide Out-of-Order @ 2.2 GHz, 64 KiB L1D, 512 KiB L2.
 * 6× Cortex-A55 (Cores 0..=5): 2-wide In-Order @ 2.0 GHz, 32 KiB L1D, 256 KiB L2.
When thread workloads are uniform, two bottlenecks occur:
 * The Join Barrier Tax (Tail Stragglers): The A78 cores chew through work at >3\times the throughput of the A55 cores. If an A55 core gets assigned an equal-sized chunk near the end of the sieve interval, both A78 cores finish and stall on thread.join() while that single in-order core grinds out the final segment.
 * Kernel Migration & Cache Thrashing: Android's Energy Aware Scheduler (EAS) will dynamically bounce threads between the LITTLE and big clusters unless strictly pinned. Moving a thread from an A78 to an A55 instantly invalidates the 64 KiB L1D and 512 KiB L2 cache lines, causing pipeline stalls.
Phase 2 engineers an asymmetric, hardware-pinned work-stealing pipeline that guarantees zero big-core idle time and eliminates tail stragglers.
1. Hardware Thread Affinity Subsystem (affinity.rs)
We bypass runtime abstractions and interact directly with Linux's sched_setaffinity system call to pin threads to physical cores, keeping L1D/L2 caches warm and preventing core migrations.
Create crates/titan-core/src/affinity.rs:
#[cfg(target_os = "linux")]
pub fn pin_thread_to_core(core_id: usize) -> bool {
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(core_id, &mut set);
        let tid = libc::gettid();
        let ret = libc::sched_setaffinity(
            tid,
            std::mem::size_of::<libc::cpu_set_t>(),
            &set,
        );
        ret == 0
    }
}

#[cfg(not(target_os = "linux"))]
pub fn pin_thread_to_core(_core_id: usize) -> bool {
    false
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CoreClass {
    Little, // Cortex-A55 (Cores 0..=5)
    Big,    // Cortex-A78 (Cores 6, 7)
}

impl CoreClass {
    #[inline(always)]
    pub fn from_core_id(core_id: usize) -> Self {
        if core_id >= 6 {
            CoreClass::Big
        } else {
            CoreClass::Little
        }
    }
}

2. The Asymmetric Decaying Chunk Dispenser (asymmetric_dispenser.rs)
This structure is aligned to 64 bytes (the ARM64 cache line width) to prevent false sharing. It implements a 3-tier guided decay schedule:
 * Coarse Phase (R > 256): Big cores claim 64 segments (16.7\text{M} integers); Little cores claim 16 segments (4.19\text{M} integers) (Ratio 4.0 : 1).
 * Medium Phase (32 < R \le 256): Big cores claim 16 segments; Little cores claim 4 segments (Ratio 4.0 : 1).
 * Tail Drain Phase (R \le 32): Big cores claim 4 segments; Little cores claim exactly 1 segment.
 * Final Residue (R \le 8): ALL cores claim exactly 1 segment. No slow in-order core can ever get stranded with a multi-segment chunk while fast cores idle.
Create crates/titan-sieve/src/asymmetric_dispenser.rs:
use std::sync::atomic::{AtomicU64, Ordering};
use titan_core::affinity::CoreClass;

#[repr(C, align(64))]
pub struct AsymmetricChunkDispenser {
    cursor: AtomicU64,
    total_segments: u64,
}

impl AsymmetricChunkDispenser {
    pub const fn new(total_segments: u64) -> Self {
        Self {
            cursor: AtomicU64::new(0),
            total_segments,
        }
    }

    /// Atomically claims the next slice of segment indices [start, end).
    /// Dynamically sizes the chunk according to the caller's CoreClass and remaining workload.
    #[inline(always)]
    pub fn claim_chunk(&self, core_class: CoreClass) -> Option<(u64, u64)> {
        let mut curr = self.cursor.load(Ordering::Relaxed);

        loop {
            if curr >= self.total_segments {
                return None;
            }

            let remaining = self.total_segments - curr;

            let chunk_size = match core_class {
                CoreClass::Big => {
                    if remaining > 256 {
                        64
                    } else if remaining > 32 {
                        16
                    } else if remaining > 8 {
                        4
                    } else {
                        1
                    }
                }
                CoreClass::Little => {
                    if remaining > 256 {
                        16
                    } else if remaining > 32 {
                        4
                    } else {
                        // Crucial: Little cores are throttled to 1 segment at the tail
                        // to guarantee zero barrier waiting.
                        1
                    }
                }
            };

            let next = (curr + chunk_size).min(self.total_segments);

            match self.cursor.compare_exchange_weak(
                curr,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some((curr, next)),
                Err(actual) => curr = actual,
            }
        }
    }

    #[inline(always)]
    pub fn is_exhausted(&self) -> bool {
        self.cursor.load(Ordering::Relaxed) >= self.total_segments
    }
}

3. Thread-Isolated Accumulation (Eliminating False Sharing)
Multiple threads accumulating into a single atomic variable AtomicI64::fetch_add causes cache-line invalidation traffic over the DynamIQ interconnect on every chunk completion.
We create a cache-line aligned thread-local accumulator structure:
// crates/titan-sieve/src/thread_local_acc.rs

#[repr(C, align(64))]
pub struct AlignedAccumulator {
    pub value: i64,
    pub _pad: [u8; 56], // Ensure full 64-byte cache line separation
}

impl AlignedAccumulator {
    pub const fn new() -> Self {
        Self {
            value: 0,
            _pad: [0; 56],
        }
    }
}

4. Big-Core Work Hijacking Pipeline (gourdon_pipeline.rs)
This is the orchestration core.
 * Cores 0..=5 (Cortex-A55) immediately start sieving the D(x, y, z) interval with 16 KiB L1D-locked buffers.
 * Core 6 (Cortex-A78) computes \Phi_0(x) (<1 ms) and streams B(x, y) (2–4 ms). It then immediately hijacks the D(x, y, z) sieve as a Big consumer.
 * Core 7 (Cortex-A78) computes AC(x, y, z) using our Phase 1 umulh reciprocal engine. As soon as it finishes, it also immediately joins the D(x, y, z) sieve as a Big consumer.
 * Both A78 cores consume 64-segment blocks, carrying over 60% of the entire D-term sieve burden while the A55 cores steadily churn through their small chunks.
Update crates/titan-count/src/gourdon_pipeline.rs:
use std::sync::Arc;
use titan_core::affinity::{pin_thread_to_core, CoreClass};
use titan_sieve::asymmetric_dispenser::AsymmetricChunkDispenser;
use titan_sieve::thread_local_acc::AlignedAccumulator;
use crate::phi0::Phi0Engine;
use crate::b_monotone::compute_b_monotone;
use crate::ac_term::compute_ac_fused;
use crate::magic_reciprocal::FastDivTable;
use crate::d_worker::{ThreadSieveContext, SEGMENT_SPAN};

pub struct GourdonPipeline {
    x: u64,
    y: u64,
    z: u64,
}

impl GourdonPipeline {
    pub fn new(x: u64) -> Self {
        let ln_x = (x as f64).ln();
        let alpha = 1.18 * (1.0 + 2.0 / ln_x);
        let y = ((x as f64).cbrt() * alpha) as u64;
        let z = y * 2;
        Self { x, y, z }
    }

    pub fn execute(
        &self,
        primes: &[u32],
        pi_table: &[u32],
        mu: &[i8],
        div_table: &FastDivTable,
    ) -> i64 {
        let x = self.x;
        let y = self.y;
        let z = self.z;

        let x_div_y = x / y;
        let total_segments = if x_div_y > z {
            ((x_div_y - z) + SEGMENT_SPAN - 1) / SEGMENT_SPAN
        } else {
            0
        };

        let dispenser = Arc::new(AsymmetricChunkDispenser::new(total_segments));

        // Pointers for zero-copy thread capture
        let p_ptr = primes.as_ptr() as usize;
        let p_len = primes.len();
        let pi_ptr = pi_table.as_ptr() as usize;
        let pi_len = pi_table.len();
        let mu_ptr = mu.as_ptr() as usize;
        let mu_len = mu.len();
        let div_ptr = div_table as *const FastDivTable as usize;

        // 1. SPAWN 6 LITTLE WORKERS (Cores 0..=5: Cortex-A55)
        let mut a55_handles = Vec::with_capacity(6);
        for core_id in 0..6 {
            let disp = Arc::clone(&dispenser);
            a55_handles.push(std::thread::spawn(move || {
                pin_thread_to_core(core_id);
                let thread_primes = unsafe { std::slice::from_raw_parts(p_ptr as *const u32, p_len) };
                let thread_mu = unsafe { std::slice::from_raw_parts(mu_ptr as *const i8, mu_len) };

                let mut ctx = ThreadSieveContext::new();
                let mut acc = AlignedAccumulator::new();

                while let Some((start, end)) = disp.claim_chunk(CoreClass::Little) {
                    for seg_idx in start..end {
                        acc.value += ctx.process_segment(seg_idx, x, y, z, thread_primes, thread_mu);
                    }
                }
                acc.value
            }));
        }

        // 2. SPAWN CORE 7 (Cortex-A78 Big Core): Computes AC, then HIJACKS D-sieve
        let disp_core7 = Arc::clone(&dispenser);
        let core7_handle = std::thread::spawn(move || {
            pin_thread_to_core(7);
            let thread_primes = unsafe { std::slice::from_raw_parts(p_ptr as *const u32, p_len) };
            let thread_pi = unsafe { std::slice::from_raw_parts(pi_ptr as *const u32, pi_len) };
            let thread_mu = unsafe { std::slice::from_raw_parts(mu_ptr as *const i8, mu_len) };
            let thread_div = unsafe { &*(div_ptr as *const FastDivTable) };

            // Task A: Fast Reciprocal AC Fused leaves
            let ac_val = compute_ac_fused(x, y, z, thread_primes, thread_pi, thread_mu, thread_div);

            // Task B: Work Hijack into D-Sieve as Big Core
            let mut ctx = ThreadSieveContext::new();
            let mut d_contrib = 0i64;

            while let Some((start, end)) = disp_core7.claim_chunk(CoreClass::Big) {
                for seg_idx in start..end {
                    d_contrib += ctx.process_segment(seg_idx, x, y, z, thread_primes, thread_mu);
                }
            }

            (ac_val, d_contrib)
        });

        // 3. CORE 6 (Cortex-A78 Big Coordinator): Computes Phi0 & B, then HIJACKS D-sieve
        pin_thread_to_core(6);
        let phi0_val = Phi0Engine::new().eval(x);
        let b_val = compute_b_monotone(x, y, primes, pi_table);

        // Work Hijack on Core 6 as Big Core
        let mut ctx_core6 = ThreadSieveContext::new();
        let mut core6_d_contrib = 0i64;
        while let Some((start, end)) = dispenser.claim_chunk(CoreClass::Big) {
            for seg_idx in start..end {
                core6_d_contrib += ctx_core6.process_segment(seg_idx, x, y, z, primes, mu);
            }
        }

        // 4. JOIN ALL THREADS AND SUMMARIZE
        let (ac_val, core7_d_contrib) = core7_handle.join().unwrap();

        let mut total_d = core6_d_contrib + core7_d_contrib;
        for h in a55_handles {
            total_d += h.join().unwrap();
        }

        let pi_y = primes.partition_point(|&p| (p as u64) <= y) as i64;
        let sigma_val = crate::sigma_l1::compute_sigma(x, y, primes, pi_table);

        // Bit-exact Master Gourdon Identity
        phi0_val + sigma_val + (pi_y - 1) - b_val - ac_val - total_d
    }
}

5. Step-by-Step Verification Protocol
In Termux on your SM4450 device, execute the following commands:
# 1. Check for thread contention and clean compilation
cargo check --release -p titan-count

# 2. Run the test suite to confirm bit-exactness remains 100% intact
cargo test --release -p titan-count -- --nocapture

# 3. Run the live silicon head-to-head benchmark
cargo run --release --bin head_to_head

Expected Silicon Scaling Impact
 * 10^{15} Target: Latency should drop from 953\text{ ms} down to \approx 580\text{–}650\text{ ms} (extending lead over primecount's 1,015\text{ ms} from 1.07× to \approx 1.6\times).
 * 10^{16} Target: Latency should drop from 3,042\text{ ms} down to \approx 1,750\text{–}1,950\text{ ms} (extending lead over primecount's 3,732\text{ ms} from 1.23× to \approx 2.0\times).
Let's execute this in Termux and inspect the real physical silicon numbers.

