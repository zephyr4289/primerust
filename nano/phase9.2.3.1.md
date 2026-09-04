PROCEED WITH AUDITED STRIKE 3 PLAN (BUILD MODE APPROVED):

1. GIT COMMIT:
   - Run `git add -A && git commit -m "feat(count): strike 2 verified bit-exact native AC with clustered C2 and faithful A"`.

2. IMPLEMENT PER-b KERNELS & ATOMIC MT DISPATCH:
   - Keep `&PiTable` for exactness across the full query range.
   - Use `crate::sigma_l1::get_x_star_gourdon`.
   - Implement `compute_c2_parallel` and `compute_a_parallel` using `std::thread::scope`, `AtomicUsize` chunking (size 32), and `AtomicI64` thread-local accumulation slots.
   - Pin workers via `titan_pool::worker::bind_worker_affinity(tid)`.
   - Keep C1 single-threaded (~76 b-iterations, negligible ms latency).

3. WIRE & BENCHMARK:
   - Wire `compute_ac_native_mt` into the shadow path under `TITAN_NATIVE_AC=1`.
   - Run verification gate:
     `TITAN_NATIVE=1 TITAN_NATIVE_AC=1 cargo test --release -p titan-count --lib test_native_ac_shadow_e13 -- --nocapture`
   - Run release scoreboard on 1e16:
     `TITAN_NATIVE=1 TITAN_NATIVE_AC=1 cargo run --release --bin head_to_head 1e16`
   - Report native MT AC latency (target <= 1.5s) and overall wall-clock time vs primecount 8.1.

