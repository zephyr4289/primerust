use titan_count::factor_table::FactorTable;

fn naive_gpf(mut n: u64) -> u64 {
    if n <= 1 { return 0; }
    let mut max_p = 0;
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            max_p = max_p.max(d);
            while n % d == 0 { n /= d; }
        }
        d += 1;
    }
    if n > 1 { max_p = max_p.max(n); }
    max_p
}

#[test]
fn test_factor_table_exhaustive_parity() {
    let limit = 500_000;
    let table = FactorTable::new(limit);

    assert_eq!(table.gpf(0), 0);
    assert_eq!(table.gpf(1), 0);

    for m in 2..=limit as u64 {
        let expected = naive_gpf(m);
        let actual = table.gpf(m);
        assert_eq!(
            actual, expected,
            "GPF mismatch for m = {}: expected {}, got {}",
            m, expected, actual
        );
    }
}
