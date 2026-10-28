//! Bounded waiting for a render job to leave `pending`, so a poller needs one call instead of a
//! loop of them. It holds no lock, DB handle, read transaction or blocking thread while parked:
//! it sleeps on the completion signal and re-reads the in-memory job record when woken.
use super::Jobs;
use crate::error::Result;
use serde_json::Value;
use std::{future::Future, time::Duration};
use tokio::time::{sleep_until, Instant};

/// Largest `render_frame` `waitMs`; the published schema uses the same bound.
pub const MAX_WAIT_MS: u64 = 10_000;

impl Jobs {
    /// Returns the job record as soon as its status is no longer `pending`. When `wait` elapses or
    /// `cancel` resolves (server shutdown) first, returns the record as it is then, still pending.
    /// Errors are the same as `get`, e.g. `JOB_NOT_FOUND` if the record is evicted meanwhile.
    pub async fn wait(
        &self,
        id: &str,
        wait: Duration,
        cancel: impl Future<Output = ()>,
    ) -> Result<Value> {
        let deadline = Instant::now() + wait.min(Duration::from_millis(MAX_WAIT_MS));
        // Subscribe before the first read: a completion between the read and the park still
        // marks the receiver changed, so no wake-up is lost.
        let mut done = self.done.subscribe();
        tokio::pin!(cancel);
        loop {
            let job = self.get(id).await?;
            if job["status"] != "pending" {
                return Ok(job);
            }
            tokio::select! {
                changed = done.changed() => if changed.is_err() { break },
                _ = sleep_until(deadline) => break,
                _ = &mut cancel => break,
            }
        }
        // The job may have finished exactly at the deadline; report its state now.
        self.get(id).await
    }
}
