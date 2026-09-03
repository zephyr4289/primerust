//! Phase 7.8: Titan Stage-Ectomy Decomposition Harness (stage_ectomy.rs).
//! Isolates micro-architectural cycles per unit on Cortex-A78 and Cortex-A55.

use std::fs;
use std::mem::MaybeUninit;
use crate::affinity::pin_to_core;

#[repr(C)]
struct PerfEventAttr {
    type_: u32,
    size: u32,
    config: u64,
    sample_period: u64,
    sample_type: u64,
    read_format: u64,
    flags: u64,
    wakeup_events: u32,
    bp_type: u32,
    config1: u64,
    config2: u64,
    branch_sample_type: u64,
    sample_regs_user: u64,
    sample_stack_user: u32,
    clockid: i32,
    modifier_flags: u64,
}

pub struct CycleMeter {
    fd_cycles: i32,
    core_freq_hz: f64,
    use_pmu: bool,
}

impl CycleMeter {
    pub fn for_core(core_id: usize) -> Self {
        pin_to_core(core_id);

        let freq_path = format!(
            "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq",
            core_id
        );
        let freq_khz: f64 = fs::read_to_string(&freq_path)
            .unwrap_or_else(|_| "2208000".to_string())
            .trim()
            .parse()
            .unwrap_or(2208000.0);
        let core_freq_hz = freq_khz * 1_000.0;

        let fd = Self::open_perf_counter(0); // 0 = PERF_COUNT_HW_CPU_CYCLES
        let use_pmu = fd >= 0;

        Self {
            fd_cycles: fd,
            core_freq_hz,
            use_pmu,
        }
    }

    fn open_perf_counter(config: u64) -> i32 {
        #[cfg(target_os = "linux")]
        {
            let mut attr = unsafe { MaybeUninit::<PerfEventAttr>::zeroed().assume_init() };
            attr.size = std::mem::size_of::<PerfEventAttr>() as u32;
            attr.type_ = 0; // PERF_TYPE_HARDWARE
            attr.config = config;
            attr.flags = (1 << 0) | (1 << 3); // disabled=1, exclude_kernel=1

            unsafe {
                libc::syscall(
                    libc::SYS_perf_event_open,
                    &attr as *const _,
                    0,  // pid: calling thread
                    -1, // cpu: any
                    -1, // group_fd
                    0,  // flags
                ) as i32
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = config;
            -1
        }
    }

    #[inline(always)]
    pub fn start(&self) -> u64 {
        if self.use_pmu {
            unsafe {
                libc::ioctl(self.fd_cycles, 0x2400); // PERF_EVENT_IOC_ENABLE
                let mut count = 0u64;
                libc::read(self.fd_cycles, &mut count as *mut _ as *mut libc::c_void, 8);
                count
            }
        } else {
            let ticks: u64;
            #[cfg(target_arch = "aarch64")]
            unsafe {
                core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks, options(nomem, nostack));
            }
            #[cfg(not(target_arch = "aarch64"))]
            {
                ticks = 0;
            }
            ticks
        }
    }

    #[inline(always)]
    pub fn stop(&self, start_val: u64) -> u64 {
        if self.use_pmu {
            unsafe {
                let mut count = 0u64;
                libc::read(self.fd_cycles, &mut count as *mut _ as *mut libc::c_void, 8);
                libc::ioctl(self.fd_cycles, 0x2401); // PERF_EVENT_IOC_DISABLE
                count.saturating_sub(start_val)
            }
        } else {
            let ticks: u64;
            #[cfg(target_arch = "aarch64")]
            unsafe {
                core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks, options(nomem, nostack));
            }
            #[cfg(not(target_arch = "aarch64"))]
            {
                ticks = 0;
            }
            let delta_ticks = ticks.saturating_sub(start_val);
            // Convert 19.2 MHz counter ticks directly to exact core execution cycles
            ((delta_ticks as f64 * self.core_freq_hz) / 19_200_000.0) as u64
        }
    }
}
