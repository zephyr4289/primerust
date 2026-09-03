//! Phase 3.2: Hardware Thread Affinity Subsystem (SM4450 Heterogeneous DynamIQ).
//!
//! Directly interacts with Linux's `sched_setaffinity` syscall to pin worker threads
//! to physical Cortex-A78 (big) and Cortex-A55 (LITTLE) cores.

#[cfg(target_os = "linux")]
extern "C" {
    fn sched_setaffinity(pid: i32, cpusetsize: usize, mask: *const u64) -> i32;
    fn gettid() -> i32;
}

#[cfg(target_os = "linux")]
pub fn pin_thread_to_core(core_id: usize) -> bool {
    if core_id >= 64 {
        return false;
    }
    let mask: u64 = 1u64 << core_id;
    unsafe {
        let tid = gettid();
        let ret = sched_setaffinity(tid, std::mem::size_of::<u64>(), &mask);
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
