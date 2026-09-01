//! Reading /proc/cpuinfo and CPU frequencies.

#[derive(Clone, Debug)]
pub struct Core {
    pub cpu: usize,
    pub part: Option<u32>,        // ARM "CPU part" hardware id
    pub implementer: Option<u32>,
}

pub fn part_name(part: u32) -> String {
    match part {
        0xd05 => "A55".into(),
        0xd0a => "A75".into(),
        0xd0b => "A76".into(),
        0xd0e => "A76".into(),
        0xd0c => "Neoverse-N1".into(),
        _ => format!("part-{part:#x}"),
    }
}

fn field(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?;
    let rest = rest.trim_start();
    let val = rest.strip_prefix(':')?;
    Some(val.trim().to_string())
}

/// Parse /proc/cpuinfo.
pub fn read() -> Vec<Core> {
    let txt = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut cores: Vec<Core> = Vec::new();
    let mut cur: Option<Core> = None;
    for line in txt.lines() {
        let line = line.trim();
        if let Some(v) = field(line, "processor") {
            if let Some(c) = cur.take() {
                cores.push(c);
            }
            if let Ok(id) = v.parse::<usize>() {
                cur = Some(Core {
                    cpu: id,
                    part: None,
                    implementer: None,
                });
            }
        } else if let Some(c) = cur.as_mut() {
            if let Some(v) = field(line, "CPU part") {
                c.part = u32::from_str_radix(v.trim_start_matches("0x"), 16).ok();
            } else if let Some(v) = field(line, "CPU implementer") {
                c.implementer = u32::from_str_radix(v.trim_start_matches("0x"), 16).ok();
            }
        }
    }
    if let Some(c) = cur {
        cores.push(c);
    }
    cores
}

/// Best-effort frequency reads (often root-gated on production Android).
pub fn read_cur_freq(cpu: usize) -> Option<u64> {
    std::fs::read_to_string(format!(
        "/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_cur_freq"
    ))
    .ok()?
    .trim()
    .parse::<u64>()
    .ok()
}

pub fn read_max_freq(cpu: usize) -> Option<u64> {
    std::fs::read_to_string(format!(
        "/sys/devices/system/cpu/cpu{cpu}/cpufreq/cpuinfo_max_freq"
    ))
    .ok()?
    .trim()
    .parse::<u64>()
    .ok()
}
