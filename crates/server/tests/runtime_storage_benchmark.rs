#[path = "runtime_storage_benchmark/mod.rs"]
mod runtime_storage_benchmark;

use runtime_storage_benchmark::{fixture, http, metrics, storage};
use serde_json::json;
use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "isolated release benchmark: bash scripts/task19-bench.sh"]
#[allow(clippy::assertions_on_constants)] // Runtime guard keeps debug test compilation valid.
async fn current_runtime_single_writer_benchmark() {
    assert!(!cfg!(debug_assertions), "benchmark requires --release");
    let output = PathBuf::from(std::env::var("TASK19_BENCH_OUTPUT_DIR").unwrap());
    let mut cells = Vec::new();
    let sqlite_version: String = rusqlite::Connection::open_in_memory()
        .unwrap()
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .unwrap();
    for count in [100, 5_000] {
        let mut canonical = None;
        for engine in ["sqlite", "pgonly", "pgmoka"] {
            eprintln!("Task19b: {engine}, {count} nodes: setup and direct calls");
            let measured = tokio::task::spawn_blocking(move || {
                let project = fixture::prepare(engine, count);
                storage::measure(project)
            })
            .await
            .unwrap();
            let normalized = fixture::normalized(&measured.snapshot);
            if let Some(expected) = &canonical {
                assert_eq!(&normalized, expected, "same authored domain across engines");
            } else {
                canonical = Some(normalized.clone());
            }
            let digest = metrics::digest(&serde_json::to_vec(&normalized).unwrap());
            eprintln!("Task19b: {engine}, {count} nodes: real HTTP snapshot calls");
            let http = http::measure(&measured, engine != "sqlite").await;
            let cell = json!({
                "engine": engine, "nodes": count, "failures": 0,
                "seedRevision": measured.seed_revision,
                "finalRevision": measured.snapshot.revision,
                "canonicalSnapshotSha256": digest,
                "snapshotJsonBytes": serde_json::to_vec(&measured.snapshot).unwrap().len(),
                "commit": measured.commit, "storageSnapshot": measured.direct,
                "storageHotProof": measured.hot_proof, "http": http,
                "firstPostWriteReadUs": measured.first_post_write_read_us,
            });
            metrics::print_cell(&cell);
            cells.push(cell);
            std::fs::write(
                output.join("samples.json"),
                serde_json::to_vec_pretty(&json!({
                    "schema": 3, "warmupPerGroup": metrics::WARMUP,
                    "sqliteVersion": sqlite_version,
                    "samplesPerGroup": metrics::SAMPLES,
                    "quantile": "nearest rank: sorted[ceil(p*N)-1]",
                    "cellOrder": "nodes100 then5000; SQLite, PostgreSQL, PostgreSQL+Moka",
                    "cells": cells,
                }))
                .unwrap(),
            )
            .unwrap();
        }
    }
    assert_eq!(cells.len(), 6);
    eprintln!(
        "Task19b raw samples: {}",
        output.join("samples.json").display()
    );
}
