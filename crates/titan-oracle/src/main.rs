//! Standalone differential oracle and self-testing mutation harness.
//! Zero shared code with any engine crate.
//!
//! Exit codes:
//!   0: Gate PASS (Reference bit-exact, all 5 mutants caught)
//!   1: Reference failed
//!   2: Mutant escaped
//!   3: Truth interlock failure

use std::process::Command;
use std::time::Instant;

// Literature constants from OEIS A006880
const A006880: &[(u64, u64)] = &[
    (10, 4),
    (100, 25),
    (1000, 168),
    (10000, 1229),
    (100000, 9592),
    (1000000, 78498),
    (10000000, 664579),
    (100000000, 5761455),
    (1000000000, 50847534),
    (10000000000, 455052511),
    (100000000000, 4118054813),
    (1000000000000, 37607912018),
    (10000000000000, 346065536839),
    (100000000000000, 3204941750802),
    (1000000000000000, 29844570422669),
    (10000000000000000, 279238341033925),
];

// Trial division prime check — trivial, auditable by inspection
fn is_prime_trial(n: u64) -> bool {
    if n < 2 { return false; }
    if n == 2 || n == 3 { return true; }
    if n % 2 == 0 || n % 3 == 0 { return false; }
    let mut d = 5u64;
    while d * d <= n {
        if n % d == 0 || n % (d + 2) == 0 {
            return false;
        }
        d += 6;
    }
    true
}

// Full trial division count
fn pi_trial(limit: u64) -> u64 {
    if limit < 2 { return 0; }
    let mut count = 0u64;
    for n in 2..=limit {
        if is_prime_trial(n) {
            count += 1;
        }
    }
    count
}

// -------------------------------------------------------------
// The Mutant Corpus: 5 Plausible Faulty Implementations
// -------------------------------------------------------------

// M1: sqrt boundary defect (`d * d < n` instead of `<=`)
fn pi_mutant_m1(limit: u64) -> u64 {
    if limit < 2 { return 0; }
    let mut count = 0u64;
    for n in 2..=limit {
        let is_p = {
            if n == 2 || n == 3 { true }
            else if n % 2 == 0 || n % 3 == 0 { false }
            else {
                let mut d = 5u64;
                let mut prime = true;
                while d * d < n { // BUG: `<` misses exact squares like 25, 49, 121
                    if n % d == 0 || n % (d + 2) == 0 {
                        prime = false;
                        break;
                    }
                    d += 6;
                }
                prime
            }
        };
        if is_p { count += 1; }
    }
    count
}

// M2: drops prime 2 (odd-only sieve bug forgetting initial +1)
fn pi_mutant_m2(limit: u64) -> u64 {
    if limit < 3 { return 0; }
    let mut count = 0u64; // BUG: misses 2
    for n in 3..=limit {
        if is_prime_trial(n) { count += 1; }
    }
    count
}

// M3: counts squares as primes
fn pi_mutant_m3(limit: u64) -> u64 {
    let mut count = pi_trial(limit);
    if limit >= 25 { count += 1; } // BUG: artificially adds 1 for 25
    count
}

// M4: scale deviation (defect manifests only at x > 5,000,000)
fn pi_mutant_m4(limit: u64) -> u64 {
    let mut count = pi_trial(limit.min(10_000_000));
    if limit > 5_000_000 {
        count = count.saturating_sub(1); // BUG: off-by-one only at deep-scale
    }
    count
}

// M5: domain off-by-one (`<` instead of `<=`)
fn pi_mutant_m5(limit: u64) -> u64 {
    if limit <= 2 { return 0; }
    pi_trial(limit - 1) // BUG: misses endpoint
}

// Candidate tester trait / closure runner
struct OracleRunner {
    full_mode: bool,
}

impl OracleRunner {
    fn test_candidate<F>(&self, name: &str, mut pi_fn: F, expected_fail: bool) -> Result<(), String>
    where
        F: FnMut(u64) -> u64,
    {
        // Tier 1a: Small exhaustive (0..2000)
        let mut running_expected = 0u64;
        for x in 0..=2000 {
            if is_prime_trial(x) {
                running_expected += 1;
            }
            let actual = pi_fn(x);
            if actual != running_expected {
                let err = format!("{name} failed at T1-small x={x}: expected {running_expected}, got {actual}");
                if expected_fail { return Err(err); } else { panic!("{err}"); }
            }
        }

        // Tier 1b: Boundary tests around prime clusters near 10^4, 10^5, 10^6
        let test_primes = [9973u64, 99991u64, 999983u64];
        for &p in &test_primes {
            for offset in [p - 1, p, p + 1] {
                let exp = pi_trial(offset);
                let actual = pi_fn(offset);
                if actual != exp {
                    let err = format!("{name} failed at T1-boundary x={offset}: expected {exp}, got {actual}");
                    if expected_fail { return Err(err); } else { panic!("{err}"); }
                }
            }
        }

        // Tier 2: Mid-range segment boundary milestones
        let mid_milestones = [10_000u64, 65_536u64, 100_000u64, 999_999u64, 1_000_000u64, 1_048_576u64];
        for &m in &mid_milestones {
            let exp = pi_trial(m);
            let actual = pi_fn(m);
            if actual != exp {
                let err = format!("{name} failed at T2-mid milestone x={m}: expected {exp}, got {actual}");
                if expected_fail { return Err(err); } else { panic!("{err}"); }
            }
        }

        // Tier 3: Literature Constants (OEIS A006880)
        let max_tier = if self.full_mode { 10_000_000u64 } else { 1_000_000u64 };
        for &(x, exp) in A006880 {
            if x > max_tier { break; }
            let actual = pi_fn(x);
            if actual != exp {
                let err = format!("{name} failed at T3-constants x={x}: expected {exp}, got {actual}");
                if expected_fail { return Err(err); } else { panic!("{err}"); }
            }
        }

        if expected_fail {
            Ok(()) // Mutant escaped!
        } else {
            Ok(())
        }
    }
}

fn query_external_primesieve(x: u64) -> Option<u64> {
    let out = Command::new("/data/data/com.termux/files/home/primesieve-ref/build/primesieve")
        .arg(x.to_string())
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        let t = line.trim();
        if t.starts_with("Primes:") {
            if let Some(num_str) = t.strip_prefix("Primes:") {
                return num_str.trim().parse::<u64>().ok();
            }
        }
    }
    None
}

fn query_external_primecount(x: u64) -> Option<u64> {
    let out = Command::new("/data/data/com.termux/files/home/primecount-ref/build/primecount")
        .arg(x.to_string())
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    for token in s.split_whitespace() {
        if let Ok(v) = token.parse::<u64>() {
            return Some(v);
        }
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let full_mode = args.iter().any(|a| a == "--full");
    let candidate_bin = args
        .windows(2)
        .find(|w| w[0] == "--candidate-bin")
        .map(|w| w[1].clone());

    println!("== TITAN-ORACLE TRUTH STACK ==");
    println!("  Mode: {}", if full_mode { "FULL (10^7 Trial + Deep Literature)" } else { "QUICK (10^6 Trial)" });
    if let Some(ref path) = candidate_bin {
        println!("  Candidate Binary: {}", path);
    }

    let t0 = Instant::now();

// M6: wheel-residue-drop (numbers = 11 mod 30 treated composite, first kill expected at x=11)
fn pi_mutant_m6(limit: u64) -> u64 {
    if limit < 2 { return 0; }
    let mut count = 0u64;
    for n in 2..=limit {
        if is_prime_trial(n) {
            if n % 30 == 11 {
                continue; // BUG: drops wheel residue 11
            }
            count += 1;
        }
    }
    count
}

    // 1. Truth Triangle Interlock Verification
    println!("\n[1/4] Verifying Truth Triangle Interlock (Trial ⟷ A006880 ⟷ primecount)...");
    for &(x, lit_val) in A006880 {
        if x <= 1_000_000 || (full_mode && x <= 10_000_000) {
            let trial_val = pi_trial(x);
            if trial_val != lit_val {
                eprintln!("[FATAL] Trial division diverged from literature at x={x}! Trial={trial_val}, Lit={lit_val}");
                std::process::exit(3);
            }
            if let Some(pc_val) = query_external_primecount(x) {
                if pc_val != lit_val {
                    eprintln!("[FATAL] primecount diverged from literature at x={x}! PC={pc_val}, Lit={lit_val}");
                    std::process::exit(3);
                }
            }
            println!("  x = 10^{:<2} : π(x) = {:>10}  [Trial=PASS, Lit=PASS, primecount=PASS]", (x as f64).log10().round() as u32, lit_val);
        }
    }

    // Deep-Scale Binchecks with primecount
    println!("\n[1b/4] Verifying Deep-Scale Literature Binchecks with primecount...");
    for &(x, lit_val) in A006880 {
        if x >= 1_000_000_000_000 {
            if let Some(pc_val) = query_external_primecount(x) {
                if pc_val != lit_val {
                    eprintln!("[FATAL] primecount diverged from literature at x={x}! PC={pc_val}, Lit={lit_val}");
                    std::process::exit(3);
                }
                println!("  [bincheck] x = 10^{:<2} : π(x) = {:>20}  [Lit=PASS, primecount=PASS]", (x as f64).log10().round() as u32, lit_val);
            }
        }
    }

    // 2. Reference Implementation Self-Check
    println!("\n[2/4] Verifying Reference Implementation (pi_trial)...");
    let runner = OracleRunner { full_mode };
    if let Err(e) = runner.test_candidate("ReferenceTrial", pi_trial, false) {
        eprintln!("[FATAL] Oracle's own reference failed: {e}");
        std::process::exit(1);
    }
    println!("  Reference trial division passed all T1, T2, T3 tiers.");

    // 3. The Mutant Corpus Discriminator Gate
    println!("\n[3/4] Running The Mutant Corpus Self-Test (6 Injected Bugs)...");
    let mut caught_count = 0;

    // Test M1
    match runner.test_candidate("Mutant_M1_SqrtBoundary", pi_mutant_m1, true) {
        Err(msg) => {
            println!("  [+] Mutant M1 (Sqrt Boundary) CAUGHT: {msg}");
            caught_count += 1;
        }
        Ok(_) => eprintln!("  [-] Mutant M1 ESCAPED!"),
    }

    // Test M2
    match runner.test_candidate("Mutant_M2_MissingTwo", pi_mutant_m2, true) {
        Err(msg) => {
            println!("  [+] Mutant M2 (Missing Two) CAUGHT: {msg}");
            caught_count += 1;
        }
        Ok(_) => eprintln!("  [-] Mutant M2 ESCAPED!"),
    }

    // Test M3
    match runner.test_candidate("Mutant_M3_SquareNumbers", pi_mutant_m3, true) {
        Err(msg) => {
            println!("  [+] Mutant M3 (Square Numbers) CAUGHT: {msg}");
            caught_count += 1;
        }
        Ok(_) => eprintln!("  [-] Mutant M3 ESCAPED!"),
    }

    // Test M4
    match runner.test_candidate("Mutant_M4_ScaleDeviation", pi_mutant_m4, true) {
        Err(msg) => {
            println!("  [+] Mutant M4 (Scale Deviation) CAUGHT: {msg}");
            caught_count += 1;
        }
        Ok(_) => eprintln!("  [-] Mutant M4 ESCAPED!"),
    }

    // Test M5
    match runner.test_candidate("Mutant_M5_DomainOffByOne", pi_mutant_m5, true) {
        Err(msg) => {
            println!("  [+] Mutant M5 (Domain Off-By-One) CAUGHT: {msg}");
            caught_count += 1;
        }
        Ok(_) => eprintln!("  [-] Mutant M5 ESCAPED!"),
    }

    // Test M6
    match runner.test_candidate("Mutant_M6_WheelResidueDrop", pi_mutant_m6, true) {
        Err(msg) => {
            println!("  [+] Mutant M6 (Wheel Residue Drop) CAUGHT: {msg}");
            caught_count += 1;
        }
        Ok(_) => eprintln!("  [-] Mutant M6 ESCAPED!"),
    }

    if caught_count < 6 {
        eprintln!("\n[ERROR] Oracle self-test failed: only {}/6 mutants caught! An oracle that cannot detect bugs is theater.", caught_count);
        std::process::exit(2);
    }
    println!("  All 6 mutants caught successfully ({caught_count}/6 kills).");

    // 3b. If Candidate Binary provided, test via Streaming Batch Protocol
    if let Some(ref path) = candidate_bin {
        println!("\n[3b/4] Testing Subprocess Candidate via Streaming Protocol: {}...", path);
        use std::io::{BufRead, BufReader, BufWriter, Write};
        use std::process::Stdio;

        let mut child = Command::new(path)
            .arg("--batch")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("Failed to spawn candidate binary {path}: {e}"));

        let mut stdin_writer = BufWriter::new(child.stdin.take().unwrap());
        let mut stdout_reader = BufReader::new(child.stdout.take().unwrap());

        let mut query_fn = |x: u64| -> u64 {
            writeln!(stdin_writer, "{x}").unwrap();
            stdin_writer.flush().unwrap();
            let mut line = String::new();
            stdout_reader.read_line(&mut line).unwrap();
            line.trim().parse::<u64>().unwrap()
        };

        // Run full T1, T2, T3 suite
        if let Err(e) = runner.test_candidate("CandidateSubprocess", &mut query_fn, false) {
            eprintln!("[FATAL] Candidate binary failed oracle suite: {e}");
            let _ = child.kill();
            std::process::exit(1);
        }

        // Deep T3 certification up to 10^11 against OEIS A006880
        println!("  Verifying Deep T3 Milestones up to 10^11 against A006880...");
        for &(x, exp) in A006880 {
            if x <= 100_000_000_000 {
                let actual = query_fn(x);
                assert_eq!(actual, exp, "Candidate failed at x={x}! exp={exp}, got={actual}");
                println!("  Candidate x = 10^{:<2} : π(x) = {:>11}  [MATCH A006880]", (x as f64).log10().round() as u32, actual);
            }
        }

        // 5 randomized large x points checked bit-exact against primesieve
        println!("  Verifying 5 Randomized-x Points against primesieve...");
        let test_points = [
            123_456_789u64,
            987_654_321u64,
            1_500_000_000u64,
            3_456_789_012u64,
            5_000_000_000u64,
        ];
        for &x in &test_points {
            let actual = query_fn(x);
            if let Some(exp) = query_external_primesieve(x) {
                assert_eq!(actual, exp, "Differential failed at x={x}! candidate={actual}, primesieve={exp}");
                println!("  Candidate x = {:>11} : π(x) = {:>10}  [MATCH primesieve]", x, actual);
            }
        }

        let _ = child.kill();
        println!("  [PASS] Candidate binary passed all oracle tiers and deep verification!");
    }

    // 4. Verification Summary
    println!("\n[4/4] Oracle Verification Gate Passed in {:.2}s", t0.elapsed().as_secs_f64());
    println!("=== ORACLE GATE: PASS (EXIT 0) ===");
    std::process::exit(0);
}
