//! titan-rig: Phase 0 measurement apparatus (runner + silicon probe).
//!
//! Usage:
//!   rig probe [--out device_profile.json]
//!   rig run --config session.json [--out session_out.json]
//!
//! Exit codes: 0 ok / complete, 2 usage/config error, 3 aborted (bad exit
//! or π mismatch — Law 2 applied to the apparatus).

mod minjson;
mod probe;
mod session;

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  rig probe [--out FILE]");
    eprintln!("  rig run --config FILE [--out FILE]");
    std::process::exit(2);
}

fn out_path(args: &[String]) -> Option<String> {
    let mut it = args.iter().peekable();
    while let Some(a) = it.next() {
        if a == "--out" {
            return it.next().cloned();
        }
    }
    None
}

fn emit_out(doc: &minjson::J, out: Option<String>) {
    let pretty = minjson::emit_pretty(doc);
    match out {
        None => print!("{}", pretty),
        Some(path) => {
            if let Err(e) = std::fs::write(&path, &pretty) {
                eprintln!("rig: cannot write {}: {}", path, e);
                std::process::exit(2);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }
    match args[0].as_str() {
        "probe" => {
            let mut i = 1;
            while i < args.len() {
                if args[i] == "--out" && i + 1 < args.len() {
                    i += 2;
                } else {
                    usage();
                }
            }
            let doc = probe::probe();
            emit_out(&doc, out_path(&args));
        }
        "run" => {
            let mut config_path: Option<String> = None;
            let mut i = 1;
            while i < args.len() {
                if args[i] == "--config" && i + 1 < args.len() {
                    config_path = Some(args[i + 1].clone());
                    i += 2;
                } else if args[i] == "--out" && i + 1 < args.len() {
                    i += 2;
                } else {
                    usage();
                }
            }
            let path = config_path.unwrap_or_else(|| usage());
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                eprintln!("rig: cannot read {}: {}", path, e);
                std::process::exit(2);
            });
            let (code, doc) = session::run_session(&text);
            emit_out(&doc, out_path(&args));
            std::process::exit(code);
        }
        _ => usage(),
    }
}
