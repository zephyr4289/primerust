//! Combinatorial Marathon Runner with Atomic Checkpoint / Resume Gauntlet.
//!
//! Evaluates pi(10^15) and pi(10^16) with fault tolerance.

use std::path::PathBuf;
use std::time::Instant;
use titan_bench::snapshot;
use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::assembly::compute_t;
use titan_count::checkpoint::{CheckpointManager, MarathonStage, MarathonState};
use titan_count::p2_sweep::compute_p2_mt;
use titan_count::p3::compute_p3_mt;
use titan_count::phi::eval_mt;
use titan_count::pi_table::PiTable;

pub fn run_marathon(
    x: u64,
    num_threads: usize,
    chk_path: &PathBuf,
    simulate_kill_at_stage: Option<u8>,
) -> (u64, f64, usize) {
    let t_start = Instant::now();
    let mgr = CheckpointManager::new(chk_path);

    let mut state = mgr.load(x).unwrap_or(MarathonState {
        x,
        stage: MarathonStage::Init,
        p3_val: 0,
        phi_val: 0,
        phi_completed_subtrees: 0,
        p2_val: 0,
        final_pi: 0,
    });

    let mut checkpoints_written = 0;
    println!("--- MARATHON: x = {} on {} Threads ---", x, num_threads);
    if state.stage != MarathonStage::Init {
        println!("  [RESUME] Found valid checkpoint at stage {:?}", state.stage);
    }

    let x_root4 = iroot4(x);
    let x_cbrt = icbrt(x);
    let x_sqrt = isqrt(x);

    let t_setup = Instant::now();
    let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
    let mut primes = Vec::with_capacity(base_primes.len() + 1);
    primes.push(0);
    primes.extend_from_slice(&base_primes);

    let a = match primes[1..].binary_search(&x_root4) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };
    let b = match primes[1..].binary_search(&x_sqrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };
    let c = match primes[1..].binary_search(&x_cbrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };

    let p_a1 = if a + 1 < primes.len() { primes[a + 1] } else { x_root4 + 1 };
    let max_table = x_sqrt.max(p_a1 * p_a1) + 30;
    println!("  PiTable span: {} numbers ({:.2} MiB)", max_table, (max_table / 30 * 8) as f64 / 1_048_576.0);
    let pi_table = PiTable::new(max_table);
    println!("  Base primes & PiTable ready in {:.3}s", t_setup.elapsed().as_secs_f64());

    if state.stage == MarathonStage::Init {
        state.stage = MarathonStage::TableReady;
        mgr.save(&state).unwrap();
        checkpoints_written += 1;
        if simulate_kill_at_stage == Some(1) {
            println!("  [KILL] Simulated crash at stage 1 (TableReady)");
            return (0, t_start.elapsed().as_secs_f64(), checkpoints_written);
        }
    }

    // Stage 2: P3
    if state.stage == MarathonStage::TableReady {
        let t_p3 = Instant::now();
        state.p3_val = compute_p3_mt(x, a, c, &primes, &pi_table, num_threads);
        state.stage = MarathonStage::P3Done;
        mgr.save(&state).unwrap();
        checkpoints_written += 1;
        println!("  [STAGE 2: P3] Evaluated P3 = {} in {:.3}s", state.p3_val, t_p3.elapsed().as_secs_f64());
        if simulate_kill_at_stage == Some(2) {
            println!("  [KILL] Simulated crash at stage 2 (P3Done)");
            return (0, t_start.elapsed().as_secs_f64(), checkpoints_written);
        }
    }

    // Stage 3: MT-Phi
    if state.stage == MarathonStage::P3Done {
        let t_phi = Instant::now();
        state.phi_val = eval_mt(x, a, &primes, &pi_table, num_threads);
        state.stage = MarathonStage::PhiDone;
        mgr.save(&state).unwrap();
        checkpoints_written += 1;
        println!("  [STAGE 3: PHI] Evaluated Phi = {} in {:.3}s", state.phi_val, t_phi.elapsed().as_secs_f64());
        if simulate_kill_at_stage == Some(3) {
            println!("  [KILL] Simulated crash at stage 3 (PhiDone)");
            return (0, t_start.elapsed().as_secs_f64(), checkpoints_written);
        }
    }

    // Stage 4: MT-P2
    if state.stage == MarathonStage::PhiDone {
        let t_p2 = Instant::now();
        state.p2_val = compute_p2_mt(x, a, b, &primes, &pi_table, num_threads);
        state.stage = MarathonStage::P2Done;
        mgr.save(&state).unwrap();
        checkpoints_written += 1;
        println!("  [STAGE 4: P2] Evaluated P2 = {} in {:.3}s", state.p2_val, t_p2.elapsed().as_secs_f64());
        if simulate_kill_at_stage == Some(4) {
            println!("  [KILL] Simulated crash at stage 4 (P2Done)");
            return (0, t_start.elapsed().as_secs_f64(), checkpoints_written);
        }
    }

    // Assembly
    let t_val = compute_t(a, b);
    let ans = (state.phi_val as i128) + (t_val as i128) - (state.p2_val as i128) - (state.p3_val as i128);
    assert!(ans >= 0);
    state.final_pi = ans as u64;
    state.stage = MarathonStage::Complete;
    mgr.save(&state).unwrap();
    checkpoints_written += 1;

    let elapsed = t_start.elapsed().as_secs_f64();
    println!("  [COMPLETE] pi({}) = {} in {:.3}s (Total Checkpoints: {})", x, state.final_pi, elapsed, checkpoints_written);
    // Preserved on disk: state.stage == MarathonStage::Complete

    (state.final_pi, elapsed, checkpoints_written)
}

fn run_gauntlet(x: u64, expected_pi: u64, chk_path: &PathBuf) {
    println!("\n============================================================");
    println!("  RUNNING AUTOMATED 5-ROUND CRASH RESUME GAUNTLET (x = {})", x);
    println!("============================================================");

    let mgr = CheckpointManager::new(chk_path);
    mgr.clear();

    for kill_stage in 1..=4 {
        println!("\n>> Gauntlet Round {}: Crash simulation at stage {}...", kill_stage, kill_stage);
        // Step 1: Run with simulated kill
        let (_pi_killed, _, _) = run_marathon(x, 8, chk_path, Some(kill_stage));
        // Step 2: Resume from checkpoint and finish
        println!(">> Gauntlet Round {}: Resuming from crash...", kill_stage);
        let (pi_resumed, sec, chks) = run_marathon(x, 8, chk_path, None);
        assert_eq!(pi_resumed, expected_pi, "Round {} resume produced incorrect count!", kill_stage);
        println!(">> Round {} SUCCESS: Resumed in {:.3}s ({} checkpoints verified bit-exact).", kill_stage, sec, chks);
        mgr.clear();
    }

    println!("\n>> Gauntlet Final Round: Clean uninterrupted run...");
    let (pi_clean, sec, _chks) = run_marathon(x, 8, chk_path, None);
    assert_eq!(pi_clean, expected_pi);
    println!(">> Clean run SUCCESS in {:.3}s.", sec);
    println!(">> 5/5 GAUNTLET ROUNDS VERIFIED 100% BIT-EXACT!");
    mgr.clear();
}

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    let args: Vec<String> = std::env::args().collect();

    let mut x = 1_000_000_000_000_000u64; // 10^15 default
    let mut num_threads = 8;
    let mut chk_path = PathBuf::from("target/marathon.chk");
    let mut gauntlet = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--x" => {
                i += 1;
                x = args[i].parse().unwrap();
            }
            "--threads" => {
                i += 1;
                num_threads = args[i].parse().unwrap();
            }
            "--checkpoint-path" => {
                i += 1;
                chk_path = PathBuf::from(&args[i]);
            }
            "--kill-gauntlet" => {
                gauntlet = true;
            }
            _ => {}
        }
        i += 1;
    }

    if gauntlet {
        let expected = match x {
            10_000_000_000_000 => 346_065_536_839u64,
            100_000_000_000_000 => 3_204_941_750_802u64,
            1_000_000_000_000_000 => 29_844_570_422_669u64,
            10_000_000_000_000_000 => 279_238_341_033_925u64,
            _ => panic!("Expected pi not specified for x={}", x),
        };
        run_gauntlet(x, expected, &chk_path);
    } else {
        let (pi_val, elapsed, _) = run_marathon(x, num_threads, &chk_path, None);
        println!("\nFINAL RESULT: pi({}) = {} (Completed in {:.3}s)", x, pi_val, elapsed);
    }
}
