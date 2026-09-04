//! Phase 6.8 + CI-1: Hardware Cycle Counters (telemetry.rs).
//!
//! - ARM64: `cntvct_el0` / `cntfrq_el0` (no syscall).
//! - x86_64 CI (Xeon Platinum / Emerald Rapids): `RDTSC` (no syscall).
//!   RDTSC is constant-TSC, suitable for interval deltas (not wall time).
//! - Other: fallback 0/1 so intervals saturate to 0 instead of lying.

#[inline(always)]
pub fn read_hardware_cycles() -> u64 {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let cycles: u64;
        std::arch::asm!("mrs {}, cntvct_el0", out(reg) cycles, options(nomem, nostack, preserves_flags));
        return cycles;
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        // _rdtsc is stable for delta measurement on Azure Xeon hosts.
        return core::arch::x86_64::_rdtsc();
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        0
    }
}

#[inline(always)]
pub fn read_timer_frequency() -> u64 {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let freq: u64;
        std::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq, options(nomem, nostack, preserves_flags));
        return freq.max(1);
    }
    // x86_64 TSC frequency is not directly readable; Azure Xeon hosts run
    // ~2.1-3.5GHz. Nominal 3GHz keeps to_ms() approximate but monotonic-safe.
    // For science use cycle deltas, not converted ms.
    #[cfg(target_arch = "x86_64")]
    {
        3_000_000_000
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        1
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct TermBreakdown {
    pub b_cycles: u64,
    pub ac_cycles: u64,
    pub d_cycles: u64,
}

impl TermBreakdown {
    pub fn to_ms(&self, freq: u64) -> (f64, f64, f64) {
        let f = freq as f64;
        (
            (self.b_cycles as f64 * 1000.0) / f,
            (self.ac_cycles as f64 * 1000.0) / f,
            (self.d_cycles as f64 * 1000.0) / f,
        )
    }
}

/// Native ARM64 Low-Power Wait For Event
#[inline(always)]
pub fn arm64_wfe() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        std::arch::asm!("wfe", options(nomem, nostack, preserves_flags));
    }
    #[cfg(not(target_arch = "aarch64"))]
    std::hint::spin_loop();
}

/// Native ARM64 Send Event to wake sleeping cores
#[inline(always)]
pub fn arm64_sev() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        std::arch::asm!("sev", options(nomem, nostack, preserves_flags));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_cycles_monotonic() {
        let c1 = read_hardware_cycles();
        let freq = read_timer_frequency();
        let c2 = read_hardware_cycles();

        #[cfg(target_arch = "aarch64")]
        {
            assert!(c2 >= c1, "Cycles must be monotonically non-decreasing");
            assert!(freq > 0, "Timer frequency must be non-zero");
        }

        let breakdown = TermBreakdown {
            b_cycles: freq,
            ac_cycles: freq * 2,
            d_cycles: freq / 2,
        };
        let (b_ms, ac_ms, d_ms) = breakdown.to_ms(freq);
        assert!((b_ms - 1000.0).abs() < 1e-3);
        assert!((ac_ms - 2000.0).abs() < 1e-3);
        assert!((d_ms - 500.0).abs() < 1e-3);
    }
}
