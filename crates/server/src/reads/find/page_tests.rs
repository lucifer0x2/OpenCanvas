//! Every byte boundary around each possible page prefix, versus the old paginator.
use super::*;

#[test]
fn bounded_prefix_matches_full_pagination_at_each_byte_boundary() {
    let scope = "f".repeat(64);
    let entries = vec![
        json!({"id":"first","text":"á日🙂".repeat(11)}),
        json!({"id":"middle","props":{"text":"x".repeat(220)}}),
        json!({"id":"last","text":"Tail"}),
    ];
    let full: Vec<_> = entries
        .iter()
        .cloned()
        .map(|v| ("nodes".into(), v))
        .collect();
    let response = json!({"nodes":[],"scannedNodes":99,"totalMatches":3});
    for offset in 0..=4 {
        for limit in [0, 1, 2, 10] {
            // Enumerate every byte, including metadata-too-large, first-item-too-large,
            // exact fits, byte-limited prefixes and complete final pages.
            for budget in 50..=950 {
                let mut bounded = Matches::new(offset, limit, budget);
                for value in &entries {
                    bounded.push(|| value.clone()).unwrap();
                }
                let actual = bounded
                    .finish(response.clone(), &scope)
                    .unwrap_or_else(|e| e.value());
                let expected = pagination::page(
                    response.clone(),
                    full.clone(),
                    offset,
                    limit,
                    budget,
                    &scope,
                )
                .unwrap_or_else(|e| e.value());
                assert_eq!(
                    actual, expected,
                    "offset={offset} limit={limit} budget={budget}"
                );
            }
        }
    }
}
