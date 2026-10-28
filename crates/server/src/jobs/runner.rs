//! The single render loop: takes the next queued job (requested tier first), runs it on the
//! shared worker process under the job deadline, and records the outcome.
use super::{png, queue::Work, Jobs};
use crate::{
    error::{ApiError, Result},
    state::Config,
    worker::Worker,
};
use serde_json::{json, Value};
use std::{
    sync::{Arc, Weak},
    time::Instant,
};
use tokio::sync::{broadcast, mpsc};

/// Runs until `Jobs` is dropped, which closes the doorbell. A ring means "the queue may have
/// work"; the loop drains the queue before parking on the doorbell again.
pub(super) fn spawn(
    jobs: Weak<Jobs>,
    mut doorbell: mpsc::Receiver<()>,
    config: Arc<Config>,
    events: broadcast::Sender<Value>,
) {
    tokio::spawn(async move {
        let mut worker: Option<Worker> = None;
        'rings: while doorbell.recv().await.is_some() {
            loop {
                let Some(owner) = jobs.upgrade() else {
                    break 'rings;
                };
                let Some(work) = owner.table.lock().await.queue.pop() else {
                    break;
                };
                drop(owner);
                let started = Instant::now();
                let result = run(&config, &mut worker, &work).await;
                let Some(owner) = jobs.upgrade() else {
                    break 'rings;
                };
                owner
                    .finish(work.id, result, started.elapsed(), &events)
                    .await;
            }
        }
        if let Some(mut w) = worker {
            w.stop().await;
        }
    });
}

async fn run(
    config: &Config,
    worker: &mut Option<Worker>,
    work: &Work,
) -> Result<(Value, Result<Value>)> {
    let result = tokio::time::timeout(config.job_timeout, async {
        if worker.is_none() {
            *worker = Some(Worker::start(config)?);
        }
        let w = worker.as_mut().unwrap();
        let mut computed = w
            .call(json!({"op":"compute","input":work.input,"strict":false}))
            .await?;
        computed["scene"]["provenance"]["sourceSceneKey"] = json!(work.key);
        let rendered = w
            .call(json!({"op":"render","scene":computed["scene"],"scale":work.scale,"strict":true}))
            .await
            .and_then(|result| {
                png::decode(&result)?;
                Ok(result)
            });
        Ok::<_, ApiError>((computed, rendered))
    })
    .await
    .unwrap_or_else(|_| {
        Err(ApiError::new(
            "JOB_TIMEOUT",
            "Render job exceeded its deadline",
        ))
    });
    // Restart after any failure, including a strict export/resource failure.
    let unhealthy = result.as_ref().map_or(true, |(_, r)| r.is_err());
    if unhealthy {
        if let Some(mut w) = worker.take() {
            w.stop().await;
        }
    }
    result
}
