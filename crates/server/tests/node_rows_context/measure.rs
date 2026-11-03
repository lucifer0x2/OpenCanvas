use super::{fixture, items, pages, Server};
use serde_json::json;
use std::time::Instant;

#[tokio::test]
async fn measure_context_rows_on_120_tiles_and_120_unused_tokens() {
    let server = Server::start().await;
    let project = fixture(&server, 120).await;
    let modes = [
        ("default", false, false),
        ("rows", false, true),
        ("compact", true, false),
        ("compact+rows", true, true),
    ];
    let mut sizes = [0; 4];
    let mut timings = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut counts = [0; 4];
    for round in 0..=8 {
        for step in 0..modes.len() {
            let mode = (step + round) % modes.len();
            let (_, compact, rows) = modes[mode];
            let mut args = json!({"projectId":project,"frameId":"frame","compact":compact});
            if rows {
                args["nodeFormat"] = json!("rows");
            }
            let start = Instant::now();
            let result = pages(&server, args).await;
            if round > 0 {
                timings[mode].push(start.elapsed());
            }
            assert_eq!(items(&result, "nodes").len(), 123);
            sizes[mode] = result
                .iter()
                .map(|page| serde_json::to_vec(page).unwrap().len())
                .sum();
            counts[mode] = result.len();
        }
    }
    for (mode, (name, _, _)) in modes.iter().enumerate() {
        timings[mode].sort_unstable();
        eprintln!(
            "{name}: {} pages, {} B, median roundtrip {:.2?}",
            counts[mode],
            sizes[mode],
            (timings[mode][3] + timings[mode][4]) / 2
        );
    }
    assert!(sizes[1] < sizes[0] && sizes[3] < sizes[2]);
}
