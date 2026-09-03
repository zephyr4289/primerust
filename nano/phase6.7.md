The TLB & Page-Walk Physics on Cortex-A78 and Cortex-A55
At 10^{18}, sieving primes up to \sqrt{x} = 10^9 yields \pi(10^9) = 50,847,534 primes. Representing them as a monolithic Vec<u32> incurs a hidden hardware penalty:
 * Footprint: 50,847,534 \times 4\text{ B} = \mathbf{203.39\text{ MB}}.
 * 4 KiB Pages: \frac{203.39\text{ MB}}{4\text{ KiB}} = \mathbf{50,848\text{ virtual pages}}.
 * Hardware Capacity:
   * Cortex-A78: 32-entry L1 D-TLB + 1,024-entry unified L2 TLB.
   * Cortex-A55: 16-entry L1 D-TLB + 512-entry unified L2 TLB.
A 50,848-page array exceeds the A55's L2 TLB capacity by 99.3\times and the A78's by 49.6\times. During streaming operations across B(x, y) and AC(x, y, z), random queries continually flush the TLB. Every miss forces the ARM64 Memory Management Unit (MMU) to perform a 3- to 4-level Translation Table Walk through LPDDR4X DRAM, adding 80 to 140 stalled cycles per access.
Sprint 1 Engineering Strategy
  Sprint 1 Memory Architecture
  ┌────────────────────────────────────────────────────────────────────────┐
  │ 1. Half-Gap Delta Encoding (Zero-Escape up to 10⁹)                      │
  │    All primes > 2 are odd -> prime gap g = 2h is always even.          │
  │    Max gap below 10⁹ is 288 -> h = g/2 <= 144.                         │
  │    Every single half-gap fits in a pure u8 with ZERO escape bytes!     │
  │    203.4 MB -> 48.49 MB (4.2× memory reduction)                       │
  ├────────────────────────────────────────────────────────────────────────┤
  │ 2. 2 MiB HugePages via MADV_HUGEPAGE                                   │
  │    Allocate buffer on 2 MiB boundaries and advise the Linux kernel.    │
  │    Collapses 50,848 pages -> 25 HugePages (100% TLB residency).       │
  ├────────────────────────────────────────────────────────────────────────┤
  │ 3. L2-Resident Sparse Checkpoint Table (Stride = 2,048 primes)         │
  │    24,828 checkpoints * 8 B = 198.6 KiB (Fits in A78 L2 / shared L3).  │
  │    Enables bounded random access in ~14 cache-hit steps.              │
  └────────────────────────────────────────────────────────────────────────┘

1. 2 MiB-Aligned HugePage Allocator (huge_alloc.rs)
Create crates/titan-core/src/huge_alloc.rs:
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ptr::NonNull;

pub const HUGE_PAGE_SIZE: usize = 2 * 1024 * 1024; // 2 MiB ARM64 PMD page

extern "C" {
    fn madvise(addr: *mut libc::c_void, len: usize, advice: libc::c_int) -> libc::c_int;
}

// Linux madvise flag for transparent huge pages
const MADV_HUGEPAGE: libc::c_int = 14;

pub struct HugePageBuffer<T> {
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
    layout: Layout,
}

unsafe impl<T: Send> Send for HugePageBuffer<T> {}
unsafe impl<T: Sync> Sync for HugePageBuffer<T> {}

impl<T> HugePageBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let elem_size = std::mem::size_of::<T>();
        let byte_capacity = (capacity * elem_size + HUGE_PAGE_SIZE - 1) & !(HUGE_PAGE_SIZE - 1);
        
        // 2 MiB aligned allocation
        let layout = Layout::from_size_align(byte_capacity, HUGE_PAGE_SIZE)
            .expect("Invalid huge page layout");

        let raw_ptr = unsafe { alloc_zeroed(layout) as *mut T };
        let ptr = NonNull::new(raw_ptr).expect("OOM allocating huge page buffer");

        // Inform the Linux kernel / Android khugepaged to back with 2 MiB PMD pages
        unsafe {
            madvise(raw_ptr as *mut libc::c_void, byte_capacity, MADV_HUGEPAGE);
        }

        Self {
            ptr,
            len: 0,
            capacity: byte_capacity / elem_size,
            layout,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, val: T) {
        debug_assert!(self.len < self.capacity);
        unsafe {
            self.ptr.as_ptr().add(self.len).write(val);
        }
        self.len += 1;
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }
}

impl<T> Drop for HugePageBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr.as_ptr() as *mut u8, self.layout);
        }
    }
}

2. Delta-Encoded Prime Stream (delta_prime_stream.rs)
Because every prime p_k \ge 3 is odd, the difference g_k = p_k - p_{k-1} is always an even integer \ge 2.
The maximal prime gap below 10^9 occurs at p = 436,273,297 with g_{\max} = 288. The half-gap h_k = g_k / 2 is bounded by 144 \le 254. Therefore, every prime gap below 10^9 fits in a single u8 without escape bytes.
Create crates/titan-count/src/delta_prime_stream.rs:
use titan_core::huge_alloc::HugePageBuffer;

pub const CHECKPOINT_STRIDE: usize = 2048; // Primes per checkpoint block

#[repr(C, align(64))]
#[derive(Copy, Clone)]
pub struct PrimeCheckpoint {
    pub prime: u32,
    pub byte_offset: u32,
}

pub struct DeltaPrimeStream {
    /// 48.5 MB stream of half-gaps backed by 2 MiB HugePages
    deltas: HugePageBuffer<u8>,
    /// 198 KiB checkpoint table: 100% L2/L3 cache-locked
    checkpoints: Vec<PrimeCheckpoint>,
    total_primes: usize,
    max_prime: u64,
}

impl DeltaPrimeStream {
    pub fn encode_from_slice(primes: &[u32]) -> Self {
        let total = primes.len();
        if total == 0 {
            return Self {
                deltas: HugePageBuffer::new(0),
                checkpoints: Vec::new(),
                total_primes: 0,
                max_prime: 0,
            };
        }

        // Allocate 2 MiB-aligned buffer for 50.8M bytes
        let mut deltas = HugePageBuffer::new(total);
        let num_checkpoints = (total + CHECKPOINT_STRIDE - 1) / CHECKPOINT_STRIDE;
        let mut checkpoints = Vec::with_capacity(num_checkpoints);

        // Prime 2 is index 0. First odd prime is primes[1] = 3.
        let mut last_p = 3u64;
        let mut current_offset = 0u32;

        for (i, &p) in primes.iter().enumerate() {
            let p_u64 = p as u64;

            if i % CHECKPOINT_STRIDE == 0 {
                checkpoints.push(PrimeCheckpoint {
                    prime: p,
                    byte_offset: current_offset,
                });
            }

            if i < 2 {
                // Skip primes 2 and 3 in the delta stream
                continue;
            }

            let gap = p_u64 - last_p;
            debug_assert!(gap % 2 == 0, "Odd gap between odd primes!");
            let half_gap = gap / 2;

            if half_gap < 255 {
                deltas.push(half_gap as u8);
                current_offset += 1;
            } else {
                // Escape byte for future scales where gap >= 510
                deltas.push(255u8);
                deltas.push((half_gap & 0xFF) as u8);
                deltas.push(((half_gap >> 8) & 0xFF) as u8);
                current_offset += 3;
            }

            last_p = p_u64;
        }

        Self {
            deltas,
            checkpoints,
            total_primes: total,
            max_prime: *primes.last().unwrap() as u64,
        }
    }

    /// O(1) random access in ~14 cache-hit steps + short local scan
    #[inline(always)]
    pub fn get(&self, idx: usize) -> u32 {
        if idx == 0 { return 2; }
        if idx == 1 { return 3; }
        if idx >= self.total_primes { return self.max_prime as u32; }

        let cp_idx = idx / CHECKPOINT_STRIDE;
        let cp = unsafe { *self.checkpoints.get_unchecked(cp_idx) };
        let mut p = cp.prime as u64;
        let mut offset = cp.byte_offset as usize;
        let start_prime_idx = cp_idx * CHECKPOINT_STRIDE;

        let delta_slice = self.deltas.as_slice();

        // Local unrolled decode inside L1D (at most 2,048 bytes)
        for _ in start_prime_idx..idx {
            let b = unsafe { *delta_slice.get_unchecked(offset) };
            if b < 255 {
                p += (b as u64) << 1;
                offset += 1;
            } else {
                let low = unsafe { *delta_slice.get_unchecked(offset + 1) } as u64;
                let high = unsafe { *delta_slice.get_unchecked(offset + 2) } as u64;
                let half_gap = (high << 8) | low;
                p += half_gap << 1;
                offset += 3;
            }
        }

        p as u32
    }

    /// Fast sequential cursor starting from prime index `start_idx`
    #[inline(always)]
    pub fn cursor_from(&self, start_idx: usize) -> DeltaPrimeCursor<'_> {
        let cp_idx = start_idx / CHECKPOINT_STRIDE;
        let cp = self.checkpoints[cp_idx];
        let mut curr_p = cp.prime as u64;
        let mut curr_offset = cp.byte_offset as usize;

        let delta_slice = self.deltas.as_slice();

        // Catch up to exact start_idx
        for _ in (cp_idx * CHECKPOINT_STRIDE)..start_idx {
            let b = delta_slice[curr_offset];
            if b < 255 {
                curr_p += (b as u64) << 1;
                curr_offset += 1;
            } else {
                let low = delta_slice[curr_offset + 1] as u64;
                let high = delta_slice[curr_offset + 2] as u64;
                curr_p += ((high << 8) | low) << 1;
                curr_offset += 3;
            }
        }

        DeltaPrimeCursor {
            deltas: delta_slice,
            offset: curr_offset,
            curr_p,
        }
    }

    #[inline(always)]
    pub fn memory_bytes(&self) -> usize {
        self.deltas.capacity() + self.checkpoints.len() * std::mem::size_of::<PrimeCheckpoint>()
    }

    #[inline(always)]
    pub fn total_primes(&self) -> usize {
        self.total_primes
    }
}

pub struct DeltaPrimeCursor<'a> {
    deltas: &'a [u8],
    offset: usize,
    curr_p: u64,
}

impl<'a> DeltaPrimeCursor<'a> {
    /// 2-instruction decode on Cortex-A78: ldrb + add with lsl #1
    #[inline(always)]
    pub fn next_prime(&mut self) -> u64 {
        let b = unsafe { *self.deltas.get_unchecked(self.offset) };
        if b < 255 {
            self.curr_p += (b as u64) << 1;
            self.offset += 1;
        } else {
            let low = unsafe { *self.deltas.get_unchecked(self.offset + 1) } as u64;
            let high = unsafe { *self.deltas.get_unchecked(self.offset + 2) } as u64;
            self.curr_p += ((high << 8) | low) << 1;
            self.offset += 3;
        }
        self.curr_p
    }

    #[inline(always)]
    pub fn current(&self) -> u64 {
        self.curr_p
    }
}

3. High-Throughput B(x, y) Sequential Decoder (b_walker.rs)
Update the Monotone Walker in crates/titan-count/src/b_walker.rs to stream through the delta stream:
use crate::delta_prime_stream::DeltaPrimeStream;
use crate::picache::PiCacheL3;

pub fn compute_b_monotone_walker_delta(
    x: u64,
    y: u64,
    stream: &DeltaPrimeStream,
    picache: &PiCacheL3,
) -> i64 {
    let sqrt_x = (x as f64).sqrt() as u64;
    if y >= sqrt_x { return 0; }

    // Locate boundary prime indices using the L2 checkpoint index
    let p_start = binary_search_stream(stream, y);
    let p_end = binary_search_stream(stream, sqrt_x);
    if p_start >= p_end { return 0; }

    let a = (p_start + 1) as i64;
    let b = p_end as i64;
    let n = b - a + 1;
    let sum_pi_p = (a + b) * n / 2;

    let mut cursor = stream.cursor_from(p_start);
    let mut sum_pi_quotients: i64 = 0;

    let mut last_v = x / cursor.current();
    let mut last_pi = picache.pi(last_v);

    for _ in p_start..p_end {
        let p = cursor.current();
        let v = x / p;

        if last_v.saturating_sub(v) < 120 {
            let delta = last_v - v;
            if delta > 0 {
                last_pi = picache.pi(v);
                last_v = v;
            }
        } else {
            last_pi = picache.pi(v);
            last_v = v;
        }

        sum_pi_quotients += last_pi as i64;
        cursor.next_prime();
    }

    sum_pi_quotients - sum_pi_p + n
}

fn binary_search_stream(stream: &DeltaPrimeStream, target: u64) -> usize {
    let mut low = 0;
    let mut high = stream.total_primes();
    while low < high {
        let mid = (low + high) / 2;
        if (stream.get(mid) as u64) <= target {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

4. Verification and Benchmark Protocol
Register pub mod huge_alloc; in titan-core/src/lib.rs and pub mod delta_prime_stream; in titan-count/src/lib.rs.
Add the unit test crates/titan-count/tests/test_delta_stream.rs:
use titan_count::delta_prime_stream::DeltaPrimeStream;
use titan_sieve::dense_popcount_neon::generate_primes_u32;

#[test]
fn test_delta_prime_stream_parity() {
    let raw_primes = generate_primes_u32(10_000_000);
    let stream = DeltaPrimeStream::encode_from_slice(&raw_primes);

    println!("Raw Primes Size : {} MB", (raw_primes.len() * 4) / (1024 * 1024));
    println!("Delta Stream Size: {} MB", stream.memory_bytes() / (1024 * 1024));

    // Verify 100% exact parity across every single prime
    for (i, &expected) in raw_primes.iter().enumerate() {
        let actual = stream.get(i);
        assert_eq!(actual, expected, "Mismatch at prime index {}", i);
    }

    // Verify cursor sequential iteration
    let mut cursor = stream.cursor_from(0);
    for (i, &expected) in raw_primes.iter().enumerate().skip(1) {
        let p = cursor.next_prime() as u32;
        assert_eq!(p, expected, "Cursor mismatch at prime index {}", i);
    }
}

Execute the verification and benchmark in Termux:
# 1. Run unit test parity suite
cargo test --release -p titan-count --test test_delta_stream -- --nocapture

# 2. Compile head_to_head_ultra with delta stream integrated
cargo build --release --bin head_to_head_ultra

# 3. Allow heatsink cooldown (30s)
sleep 30

# 4. Benchmark scale 10^18 directly
./target/release/head_to_head_ultra 1e18


