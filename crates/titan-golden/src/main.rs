// Golden oracle driver: calls the INSTRUMENTED oracle libprimecount.a
// directly with explicit (x, y, z), bypassing the CLI alpha pipeline
// (whose truncate3/round steps cannot reproduce our exact y/z).
//
// Term sums print to stdout (exact decimal). With PC_DUMP=1, per-unit
// partials go to stderr as ORACLE lines (captured by rig sessions).
// With ORACLE_TIMERS=1, region timers go to stderr.
// Pinning/threads via PC_PIN, PC_THREADS_{AC,B,D} (inherited env).
//
// Usage: golden <AC|B|D> <x> <y> <z> [--threads N]
// Exit 0 + sum on stdout; nonzero on usage/linkage failure.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!("usage: golden <AC|B|D> <x> <y> <z> [--threads N]");
        std::process::exit(2);
    }
    let term = args[0].clone();
    let x: u64 = args[1].parse().unwrap_or_else(|_| usage());
    let y: u64 = args[2].parse().unwrap_or_else(|_| usage());
    let z: u64 = args[3].parse().unwrap_or_else(|_| usage());
    let mut threads: i32 = 8;
    let mut i = 4;
    while i < args.len() {
        if args[i] == "--threads" && i + 1 < args.len() {
            threads = args[i + 1].parse().unwrap_or_else(|_| usage());
            i += 2;
        } else {
            usage();
        }
    }

    extern "C" {
        #[link_name = "_ZN10primecount2ACEllllib"]
        fn primecount_ac_64(x: i64, y: i64, z: i64, k: i64, threads: i32, is_print: bool) -> i64;
        #[link_name = "_ZN10primecount1BEllib"]
        fn primecount_b_64(x: i64, y: i64, threads: i32, is_print: bool) -> i64;
        #[link_name = "_ZN10primecount1DElllllib"]
        fn primecount_d_64(
            x: i64,
            y: i64,
            z: i64,
            k: i64,
            x_star: i64,
            threads: i32,
            is_print: bool,
        ) -> i64;
    }

    // Same x_star construction the pipeline uses (get_x_star_gourdon port).
    // Nested fns keep this binary dependency-free; checked ops (no debug overflow).
    fn iroot4(x: u64) -> u64 {
        let mut r = (x as f64).powf(0.25) as u64;
        while (r + 1).checked_pow(4).unwrap_or(u64::MAX) <= x {
            r += 1;
        }
        while r.checked_pow(4).unwrap_or(u64::MAX) > x {
            r -= 1;
        }
        r
    }
    fn isqrt(x: u64) -> u64 {
        let mut r = (x as f64).sqrt() as u64;
        while (r + 1).checked_mul(r + 1).unwrap_or(u64::MAX) <= x {
            r += 1;
        }
        while r.checked_mul(r).unwrap_or(u64::MAX) > x {
            r -= 1;
        }
        r
    }
    fn x_star_of(x: u64, y: u64) -> u64 {
        let y = y.max(1);
        let yy = (y as u128) * (y as u128);
        let x_div_yy = ((x as u128 + yy - 1) / yy) as u64;
        iroot4(x).max(x_div_yy).min(y).min(isqrt(x / y)).max(1)
    }

    let ans: i64 = unsafe {
        match term.as_str() {
            "AC" => primecount_ac_64(x as i64, y as i64, z as i64, 8, threads, false),
            "B" => primecount_b_64(x as i64, y as i64, threads, false),
            "D" => {
                let xs = x_star_of(x, y);
                primecount_d_64(x as i64, y as i64, z as i64, 8, xs as i64, threads, false)
            }
            _ => usage(),
        }
    };
    println!("{}", ans);
}

fn usage() -> ! {
    eprintln!("usage: golden <AC|B|D> <x> <y> <z> [--threads N]");
    std::process::exit(2);
}
