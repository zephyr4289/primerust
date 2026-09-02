//! Gate Contract Runner: Law 0 Enforcer & Retro-Audit Evaluator.
//!
//! Loads criteria from bench/contracts/phase{N}.json, reports status
//! per line, and exits with code = non_pass_count.

use std::fs;
use std::path::Path;

#[derive(Debug)]
struct Criterion {
    id: String,
    description: String,
    status: String,
}

#[derive(Debug)]
struct PhaseContract {
    phase: u32,
    name: String,
    criteria: Vec<Criterion>,
}

fn parse_contract<P: AsRef<Path>>(path: P) -> Option<PhaseContract> {
    let content = fs::read_to_string(path).ok()?;
    let mut phase = 0u32;
    let mut name = String::new();
    let mut criteria = Vec::new();

    // Lightweight JSON parser
    let mut in_criteria = false;
    let mut cur_id = String::new();
    let mut cur_desc = String::new();
    let mut cur_status = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"phase\":") {
            let val = trimmed.trim_start_matches("\"phase\":").trim().trim_matches(',');
            phase = val.parse().unwrap_or(0);
        } else if trimmed.starts_with("\"name\":") {
            let val = trimmed.trim_start_matches("\"name\":").trim().trim_matches(&[',', '"'][..]);
            name = val.to_string();
        } else if trimmed.starts_with("\"criteria\":") {
            in_criteria = true;
        } else if in_criteria {
            if trimmed.starts_with("\"id\":") {
                cur_id = trimmed.trim_start_matches("\"id\":").trim().trim_matches(&[',', '"'][..]).to_string();
            } else if trimmed.starts_with("\"description\":") {
                cur_desc = trimmed.trim_start_matches("\"description\":").trim().trim_matches(&[',', '"'][..]).to_string();
            } else if trimmed.starts_with("\"status\":") {
                cur_status = trimmed.trim_start_matches("\"status\":").trim().trim_matches(&[',', '"'][..]).to_string();
            } else if trimmed.starts_with('}') && !cur_id.is_empty() {
                criteria.push(Criterion {
                    id: cur_id.clone(),
                    description: cur_desc.clone(),
                    status: cur_status.clone(),
                });
                cur_id.clear();
                cur_desc.clear();
                cur_status.clear();
            }
        }
    }

    Some(PhaseContract { phase, name, criteria })
}

fn eval_phase(phase: u32) -> (usize, usize, usize) {
    let path = format!("bench/contracts/phase{}.json", phase);
    let contract = match parse_contract(&path) {
        Some(c) => c,
        None => {
            eprintln!("Failed to read contract: {}", path);
            return (0, 0, 1);
        }
    };

    println!("\n=== PHASE {} GATE CONTRACT: {} ===", contract.phase, contract.name);
    let mut pass = 0;
    let mut fail = 0;
    let mut owed = 0;

    for c in &contract.criteria {
        let tag = match c.status.as_str() {
            "PASS" => { pass += 1; "\x1b[32m[PASS]\x1b[0m" },
            "FAIL" => { fail += 1; "\x1b[31m[FAIL]\x1b[0m" },
            "OWED" => { owed += 1; "\x1b[33m[OWED]\x1b[0m" },
            _ => "\x1b[31m[UNKNOWN]\x1b[0m",
        };
        println!("  {:>6} {:<7} : {}", c.id, tag, c.description);
    }

    let non_pass = fail + owed;
    println!("  Summary: {} PASS, {} FAIL, {} OWED (Non-PASS: {})", pass, fail, owed, non_pass);
    (pass, fail, owed)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--all") {
        println!("============================================================");
        println!("         TITAN HONEST RETRO-AUDIT SCOREBOARD (LAW 0)        ");
        println!("============================================================");

        let mut total_pass = 0;
        let mut total_fail = 0;
        let mut total_owed = 0;

        for p in 0..=35 {
            if Path::new(&format!("bench/contracts/phase{}.json", p)).exists() {
                let (pass, fail, owed) = eval_phase(p);
                total_pass += pass;
                total_fail += fail;
                total_owed += owed;
            }
        }

        println!("\n============================================================");
        println!("PROJECT RETRO-AUDIT SUMMARY (PHASES 0 - 35):");
        println!("  Total Certified Criteria : {}", total_pass + total_fail + total_owed);
        println!("  PASS                     : {}", total_pass);
        println!("  FAIL                     : {}", total_fail);
        println!("  OWED                     : {}", total_owed);
        let pct = (total_pass as f64) / ((total_pass + total_fail + total_owed) as f64) * 100.0;
        println!("  True Completion Rate     : {:.1}%", pct);
        println!("============================================================");

        if total_fail + total_owed > 0 {
            std::process::exit(1);
        } else {
            std::process::exit(0);
        }
    }

    let phase: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(7);
    let (_p, fail, owed) = eval_phase(phase);
    let code = (fail + owed) as i32;
    std::process::exit(code);
}
