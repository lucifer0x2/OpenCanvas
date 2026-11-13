//! Warm-up/sample loops and nearest-rank summaries in milliseconds.
use serde_json::{json, Value};
use std::time::Instant;

pub struct Plan {
    pub warmup: usize,
    pub samples: usize,
}

impl Plan {
    pub fn from_env() -> Self {
        let read = |key: &str, default: usize| {
            std::env::var(key)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        };
        let plan = Self {
            warmup: read("TASK24_WARMUP", 1),
            samples: read("TASK24_SAMPLES", 5),
        };
        assert!(plan.samples >= 5, "Task24 requires at least five samples");
        plan
    }
    pub fn total(&self) -> usize {
        self.warmup + self.samples
    }
    pub fn measured(&self, index: usize) -> bool {
        index >= self.warmup
    }
}

pub fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1_000.0
}

/// Nearest rank: sorted[ceil(p*N)-1]. Values stay in recording order.
pub fn summary(values: &[f64]) -> Value {
    assert!(!values.is_empty());
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = |p: f64| sorted[((p * sorted.len() as f64).ceil() as usize).max(1) - 1];
    json!({
        "unit": "ms", "n": values.len(), "median": rank(0.5), "p95": rank(0.95),
        "min": sorted[0], "max": sorted[sorted.len() - 1], "values": values,
    })
}

/// Times a blocking closure `plan.total()` times and summarizes the measured runs.
pub fn sample<T>(plan: &Plan, mut run: impl FnMut(usize) -> T) -> (Value, T) {
    let mut values = Vec::new();
    let mut last = None;
    for index in 0..plan.total() {
        let start = Instant::now();
        let out = run(index);
        let elapsed = ms(start);
        if plan.measured(index) {
            values.push(elapsed);
        }
        last = Some(out);
    }
    (summary(&values), last.expect("at least one run"))
}

/// Current resident set of this process in KiB, via ps (no extra dependency).
pub fn rss_kib() -> Option<u64> {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    String::from_utf8(out.stdout).ok()?.trim().parse().ok()
}
