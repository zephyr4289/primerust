//! Pre-Flight Experiment C7: MT-Phi Strategy Census (Spine-Split vs BFS Level-Banding).
//!
//! Measures:
//!   1. Spine-Split right-children subtree size distribution (top subtree skew).
//!   2. BFS frontier width per level (peak frontier memory requirement).

use std::time::Instant;
use titan_bench::snapshot;
use titan_core::roots::{iroot4, isqrt};
use titan_count::phi::PhiEngine;
use titan_count::pi_table::PiTable;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== PRE-FLIGHT EXPERIMENT C7: SUBTREE & FRONTIER CENSUS ==");

    let mut engine = PhiEngine::new();

    for &pow in &[12, 13, 14] {
        let x = 10u64.pow(pow);
        let x_root4 = iroot4(x);
        let x_sqrt = isqrt(x);

        let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let a = match primes[1..].binary_search(&x_root4) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        // Table span capped at x^1/2 (RAM Law compliant)
        let pi_table = PiTable::new(x_sqrt + 30);

        println!("\n============================================================");
        println!("  Scale 10^{} (x = {}, a = {})", pow, x, a);
        println!("============================================================");

        // 1. Spine-Split Subtree Census
        let t0 = Instant::now();
        let mut subtree_sizes = Vec::new();
        let mut total_subtree_nodes = 0u64;

        // Along the left spine: right-children are pushed for i from a down to 7
        for i in (7..=a).rev() {
            let p_i = primes[i];
            let y_child = x / p_i;
            let _val = engine.eval_with_census(y_child, i - 1, &primes, &pi_table);
            let nodes = engine.census.total_nodes;
            subtree_sizes.push((i, nodes));
            total_subtree_nodes += nodes;
        }
        let elapsed_spine = t0.elapsed().as_secs_f64();

        // Sort descending by subtree node count
        subtree_sizes.sort_by(|a, b| b.1.cmp(&a.1));

        let top1_pct = if total_subtree_nodes > 0 { (subtree_sizes[0].1 as f64) / (total_subtree_nodes as f64) * 100.0 } else { 0.0 };
        let top2_sum: u64 = subtree_sizes.iter().take(2).map(|s| s.1).sum();
        let top2_pct = if total_subtree_nodes > 0 { (top2_sum as f64) / (total_subtree_nodes as f64) * 100.0 } else { 0.0 };
        let top4_sum: u64 = subtree_sizes.iter().take(4).map(|s| s.1).sum();
        let top4_pct = if total_subtree_nodes > 0 { (top4_sum as f64) / (total_subtree_nodes as f64) * 100.0 } else { 0.0 };
        let top8_sum: u64 = subtree_sizes.iter().take(8).map(|s| s.1).sum();
        let top8_pct = if total_subtree_nodes > 0 { (top8_sum as f64) / (total_subtree_nodes as f64) * 100.0 } else { 0.0 };

        println!("  [Spine-Split Strategy]");
        println!("    Total Right-Children Dispatched : {}", subtree_sizes.len());
        println!("    Total Subtree Nodes             : {}", total_subtree_nodes);
        println!("    Time to Evaluate All Subtrees   : {:.3}s", elapsed_spine);
        println!("    Top-1 Subtree (i={:<3})           : {:>9} nodes ({:>5.2}% of total)", subtree_sizes[0].0, subtree_sizes[0].1, top1_pct);
        println!("    Top-2 Subtrees Cumulative       : {:>9} nodes ({:>5.2}% of total)", top2_sum, top2_pct);
        println!("    Top-4 Subtrees Cumulative       : {:>9} nodes ({:>5.2}% of total)", top4_sum, top4_pct);
        println!("    Top-8 Subtrees Cumulative       : {:>9} nodes ({:>5.2}% of total)", top8_sum, top8_pct);
        println!("    Smallest Non-Zero Subtree (i={}): {:>9} nodes", subtree_sizes.last().unwrap().0, subtree_sizes.last().unwrap().1);

        // 2. BFS Level-Banding Frontier Simulation
        println!("\n  [BFS Level-Banding Frontier Measurement]");
        let mut frontier = vec![x];
        let mut max_frontier_width = 1usize;
        let mut peak_level = a;

        for i in (7..=a).rev() {
            let p_i = primes[i];
            let next_p = primes[i]; // p_i
            let p_sq = next_p * next_p;

            let mut next_frontier = Vec::new();
            for &y in &frontier {
                // Left child: y at level i-1
                if y >= p_sq {
                    next_frontier.push(y);
                }
                // Right child: y / p_i at level i-1
                let y_div = y / p_i;
                if y_div >= p_sq {
                    next_frontier.push(y_div);
                }
            }

            let cur_width = next_frontier.len();
            if cur_width > max_frontier_width {
                max_frontier_width = cur_width;
                peak_level = i - 1;
            }

            frontier = next_frontier;
            if frontier.is_empty() {
                break;
            }
        }

        let frontier_mem_mb = (max_frontier_width * 16) as f64 / 1_048_576.0;
        println!("    Peak Active Frontier Width      : {:>9} nodes (at level i={})", max_frontier_width, peak_level);
        println!("    Peak Frontier Memory Footprint   : {:>6.2} MB", frontier_mem_mb);
        println!("    BFS Frontier Status             : {}", if frontier_mem_mb <= 400.0 { "VIABLE (< 400 MB)" } else { "EXCESSIVE (> 400 MB)" });
    }
}
