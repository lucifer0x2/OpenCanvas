//! Retry guidance for a full render queue: a moving average of recent successful job durations,
//! times the jobs that would still run before a request sent now.
use super::wait::MAX_WAIT_MS;
use crate::error::ApiError;
use serde_json::json;
use std::time::Duration;

const HINT: &str = "Requested renders are full. Wait retryAfterMs, then resend the same request; \
to follow a job you already have, call render_frame {jobId, waitMs} instead of a new render.";

pub struct Pace {
    average_ms: f64,
}

impl Default for Pace {
    fn default() -> Self {
        // A cold worker start plus one scene; replaced by measurements after the first job.
        Self { average_ms: 1000.0 }
    }
}

impl Pace {
    /// Records a job that produced a scene. Fast failures (an unavailable worker, a rejected
    /// input) say nothing about how long a render holds the worker, so callers skip them.
    pub fn record(&mut self, elapsed: Duration) {
        let ms = elapsed.as_secs_f64() * 1000.0;
        self.average_ms = self.average_ms * 0.7 + ms * 0.3;
    }
    /// Estimated time for `ahead` jobs (queued plus running) to finish, within 100–10,000 ms.
    pub fn retry_after_ms(&self, ahead: usize) -> u64 {
        ((self.average_ms * ahead.max(1) as f64).round() as u64).clamp(100, MAX_WAIT_MS)
    }
    /// Retryable `QUEUE_FULL` with `retryAfterMs` for `ahead` jobs and a `hint` on how to retry.
    pub fn queue_full(&self, message: &str, ahead: usize) -> ApiError {
        ApiError::new("QUEUE_FULL", message)
            .with_detail("retryAfterMs", json!(self.retry_after_ms(ahead)))
            .with_detail("hint", json!(HINT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_follows_measured_jobs_within_bounds() {
        let mut pace = Pace::default();
        assert_eq!(pace.retry_after_ms(1), 1000);
        for _ in 0..30 {
            pace.record(Duration::from_millis(300));
        }
        assert_eq!(pace.retry_after_ms(1), 300);
        for _ in 0..30 {
            pace.record(Duration::from_secs(60));
        }
        assert_eq!(pace.retry_after_ms(1), MAX_WAIT_MS);
        let error = pace.queue_full("full", 1).value();
        assert_eq!(error["error"]["code"], "QUEUE_FULL");
        assert_eq!(error["error"]["retryable"], true);
        assert_eq!(error["error"]["retryAfterMs"], MAX_WAIT_MS);
        assert!(error["error"]["hint"].as_str().unwrap().contains("waitMs"));
    }

    #[test]
    fn retry_after_counts_the_jobs_ahead() {
        let mut pace = Pace::default();
        for _ in 0..30 {
            pace.record(Duration::from_millis(150));
        }
        assert_eq!(pace.retry_after_ms(0), 150);
        assert_eq!(pace.retry_after_ms(9), 1350);
        assert_eq!(pace.retry_after_ms(100), MAX_WAIT_MS);
    }
}
