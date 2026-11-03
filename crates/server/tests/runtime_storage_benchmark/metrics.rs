use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::Instant;

pub const WARMUP: usize = 20;
pub const SAMPLES: usize = 100;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Samples {
    pub unit: &'static str,
    pub values: Vec<f64>,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub failures: usize,
}

impl Samples {
    pub fn new(values: Vec<f64>) -> Self {
        assert_eq!(values.len(), SAMPLES);
        let mut sorted = values.clone();
        sorted.sort_by(f64::total_cmp);
        let percentile = |p: f64| sorted[(p * sorted.len() as f64).ceil() as usize - 1];
        Self {
            unit: "us",
            values,
            p50: percentile(0.50),
            p95: percentile(0.95),
            p99: percentile(0.99),
            failures: 0,
        }
    }
}

pub fn elapsed_us(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1_000_000.0
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn print_cell(cell: &Value) {
    for metric in ["commit", "storageSnapshot"] {
        eprintln!(
            "Task19b {} nodes={} {metric} us p50={} p95={} p99={}",
            cell["engine"],
            cell["nodes"],
            cell[metric]["p50"],
            cell[metric]["p95"],
            cell[metric]["p99"]
        );
    }
    for format in ["full", "rows"] {
        let get = &cell["http"][format];
        for (metric, samples, bytes) in [
            ("200 total", &get["total"], &get["responseBodyBytes"]),
            (
                "304 receive",
                &get["notModified"]["receiveBody"],
                &get["notModified"]["responseBodyBytes"],
            ),
        ] {
            eprintln!(
                "Task19b {} nodes={} HTTP {format} {metric} us p50={} p95={} p99={} bodyBytes={bytes}",
                cell["engine"],
                cell["nodes"],
                samples["p50"],
                samples["p95"],
                samples["p99"],
            );
        }
    }
}
