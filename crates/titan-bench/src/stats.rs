//! Robust statistics: median + MAD (immune to Android background-jitter outliers).

#[derive(Debug, Clone)]
pub struct SampleStats {
    pub n: usize,
    pub min: f64,
    pub max: f64,
    pub median: f64,
    pub mad: f64,
    pub mean: f64,
}

pub fn describe(mut v: Vec<f64>) -> SampleStats {
    assert!(!v.is_empty(), "sample vector must not be empty");
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    let median = v[n / 2];
    let mut d: Vec<f64> = v.iter().map(|&x| (x - median).abs()).collect();
    d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mad = d[n / 2];
    let mean = v.iter().sum::<f64>() / n as f64;
    SampleStats {
        n,
        min: v[0],
        max: v[n - 1],
        median,
        mad,
        mean,
    }
}
