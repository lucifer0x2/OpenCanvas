//! Task24 large-project baseline. Built only with `--features large-project-bench`
//! (see `required-features` in Cargo.toml); run through `bash scripts/task24-bench.sh`.
#[path = "large_project_benchmark/mod.rs"]
mod large_project_benchmark;

use large_project_benchmark::cell;
use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Task24 release benchmark: bash scripts/task24-bench.sh"]
#[allow(clippy::assertions_on_constants)] // Runtime guard keeps debug test compilation valid.
async fn large_project_cell() {
    assert!(!cfg!(debug_assertions), "benchmark requires --release");
    assert!(
        opencanvas_storage::MAX_NODES > 20_000,
        "benchmark build must raise the node cap"
    );
    let output = PathBuf::from(std::env::var("TASK24_OUTPUT_DIR").unwrap());
    let cell = cell::Cell::from_env();
    let name = format!(
        "cell-{}-{:?}-{:02}.json",
        cell.engine, cell.variant, cell.copies
    );
    let result = cell::run(cell).await;
    std::fs::write(
        output.join(name.to_lowercase()),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
}
