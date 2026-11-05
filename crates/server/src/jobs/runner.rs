//! The single render loop: takes the next queued job (requested tier first), runs it on the
//! selected compute/paint backend under the job deadline, and records the outcome.
use super::{
    export::{self, Rendered},
    export_svg::AssetSource,
    painter::Painter,
    queue::Work,
    Jobs,
};
use crate::{
    error::{ApiError, Result},
    state::Config,
    worker::Worker,
    RenderBackend,
};
use opencanvas_storage::Storage;
use serde_json::{json, Value};
use std::{
    sync::{Arc, Weak},
    time::Instant,
};
use tokio::sync::{broadcast, mpsc};
#[cfg(test)]
#[path = "runner_tests.rs"]
mod tests;

/// Runs until `Jobs` is dropped, which closes the doorbell. A ring means "the queue may have
/// work"; the loop drains the queue before parking on the doorbell again.
pub(super) fn spawn(
    jobs: Weak<Jobs>,
    mut doorbell: mpsc::Receiver<()>,
    config: Arc<Config>,
    events: broadcast::Sender<Value>,
    store: Storage,
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
                let deadline = started + config.job_timeout;
                let result = run(&config, &store, &mut worker, &work, deadline).await;
                let Some(owner) = jobs.upgrade() else {
                    break 'rings;
                };
                owner
                    .finish(work.id, result, started.elapsed(), deadline, &events)
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
    store: &Storage,
    worker: &mut Option<Worker>,
    work: &Work,
    deadline: Instant,
) -> Result<(Value, Result<Rendered>)> {
    // Native owns/drains every CPU task; no outer timeout may drop its paint future.
    if config.render_backend == RenderBackend::Rust {
        return run_native(store, work, deadline).await;
    }
    let result = tokio::time::timeout_at(deadline.into(), async {
        let mut computed = worker_mut(config, worker)?
            .call(json!({"op":"compute","input":work.input,"strict":false}))
            .await?;
        prepare(&mut computed, work);
        let rendered = async {
            let mut painter = Painter::Chromium(worker_mut(config, worker)?);
            if let Some(spec) = &work.export {
                let assets = AssetSource {
                    store: store.clone(),
                    project_id: work.frame.0.clone(),
                    cached: None,
                    deadline: None,
                };
                return export::render(&mut painter, &assets, &computed, spec).await;
            }
            painter
                .paint(&computed["scene"], work.scale)
                .await
                .map(Rendered::Frame)
        }
        .await;
        Ok::<_, ApiError>((computed, rendered))
    })
    .await
    .unwrap_or_else(|_| {
        Err(ApiError::new(
            "JOB_TIMEOUT",
            "Render job exceeded its deadline",
        ))
    });
    // Restart after any failure, including a strict export/resource failure, except an export
    // rejected for its node size or scale.
    let unhealthy = result.as_ref().map_or(true, |(_, r)| {
        r.as_ref().is_err_and(|e| !export::keeps_worker(e))
    });
    if unhealthy {
        if let Some(mut w) = worker.take() {
            w.stop().await;
        }
    }
    result
}

fn prepare(computed: &mut Value, work: &Work) {
    computed["scene"]["provenance"]["sourceSceneKey"] = json!(work.key);
    super::states::apply(&mut computed["scene"], &work.input);
    crate::text_diagnostics::append(&mut computed["scene"], &work.text_overflow_intent);
}

async fn run_native(
    store: &Storage,
    work: &Work,
    deadline: Instant,
) -> Result<(Value, Result<Rendered>)> {
    let native = super::native::compute(store, work, deadline).await?;
    let mut computed = native.value;
    let provenance = &mut computed["scene"]["provenance"];
    provenance["engine"] = json!("native");
    provenance["engineVersion"] = json!("opencanvas-render-0.1.0/tiny-skia-0.12.0");
    provenance["runtimeVersion"] = json!("rust-native");
    prepare(&mut computed, work);
    let assets = AssetSource {
        store: store.clone(),
        project_id: work.frame.0.clone(),
        cached: Some(native.bytes.clone()),
        deadline: Some(deadline),
    };
    let mut painter = Painter::Native {
        bytes: native.bytes,
        deadline,
        readiness: computed["readiness"]
            .as_array()
            .cloned()
            .unwrap_or_default(),
    };
    let rendered = if let Some(spec) = &work.export {
        export::render(&mut painter, &assets, &computed, spec).await
    } else {
        painter
            .paint(&computed["scene"], work.scale)
            .await
            .map(Rendered::Frame)
    };
    let rendered = rendered.and_then(|output| {
        super::native::check_deadline(deadline)?;
        Ok(output)
    });
    computed["readiness"] = json!(painter.readiness().unwrap());
    Ok((computed, rendered))
}

fn worker_mut<'a>(config: &Config, worker: &'a mut Option<Worker>) -> Result<&'a mut Worker> {
    if worker.is_none() {
        *worker = Some(Worker::start(config)?);
    }
    Ok(worker.as_mut().unwrap())
}
