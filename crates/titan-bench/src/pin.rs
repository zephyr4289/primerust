//! Thread pinning. Own-thread affinity needs no root on Android.
//! NOTE: keep Termux in the FOREGROUND — background apps get restricted cpusets.

use std::io;

fn set_mask(mask: [u64; 16]) -> io::Result<()> {
    let rc = unsafe {
        libc::sched_setaffinity(
            0, // calling thread
            std::mem::size_of_val(&mask),
            &mask as *const [u64; 16] as *const libc::cpu_set_t,
        )
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub fn set_affinity(cpu: usize) -> io::Result<()> {
    let mut mask = [0u64; 16];
    mask[cpu / 64] |= 1u64 << (cpu % 64);
    set_mask(mask)
}

/// Restore full mask — REQUIRED before spawning threads, since children
/// inherit the creator's mask (and would all land on one core).
pub fn set_full_affinity() -> io::Result<()> {
    set_mask([u64::MAX; 16])
}

pub fn ncpus() -> usize {
    unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as usize }
}
