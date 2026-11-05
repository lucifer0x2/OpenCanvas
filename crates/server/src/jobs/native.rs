//! One in-process layout invocation. No worker, URL fetch, layout-step or OS fonts.
#[cfg(test)]
mod cache_tests;
mod resources;
#[cfg(test)]
mod tests;
use super::queue::Work;
use crate::error::{ApiError, Result};
use opencanvas_layout::{LayoutError, LayoutInput};
use opencanvas_storage::Storage;
use opencanvas_text::FontSetMeasurer;
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use tokio::sync::Semaphore;

// Also bounds detached work if a caller itself is aborted: the permit lives inside
// the blocking closure, never in the dropped async future. No blocking-job backlog.
static LANE: Semaphore = Semaphore::const_new(1);

/// Safe scene JSON boundary: diagnostics may become opaque objects after this returns.
/// Fonts are metrics-ready; native paint/inspection adds observed image decode readiness.
pub(super) async fn compute(store: &Storage, work: &Work, deadline: Instant) -> Result<Computed> {
    blocking(deadline, {
        let store = store.clone();
        let input = work.input.clone();
        move |control| compute_sync(&store, input, control)
    })
    .await
}

pub(super) async fn blocking<T: Send + 'static>(
    deadline: Instant,
    run: impl FnOnce(Control) -> Result<T> + Send + 'static,
) -> Result<T> {
    blocking_with_lane(deadline, run, &LANE).await
}

async fn blocking_with_lane<T: Send + 'static>(
    deadline: Instant,
    run: impl FnOnce(Control) -> Result<T> + Send + 'static,
    lane: &'static Semaphore,
) -> Result<T> {
    let control = Control::new(deadline);
    let _cancel_on_drop = CancelOnDrop(control.cancelled.clone());
    let until = tokio::time::Instant::from_std(deadline);
    let permit = tokio::time::timeout_at(until, lane.acquire())
        .await
        .map_err(|_| timeout())?
        .map_err(|_| ApiError::new("INTERNAL_ERROR", "Native compute lane unavailable"))?;
    check_deadline(deadline)?;
    let mut task = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        control.check()?;
        run(control)
    });
    let result = match tokio::time::timeout_at(until, &mut task).await {
        Ok(result) => {
            result.map_err(|_| ApiError::new("INTERNAL_ERROR", "Native compute failed"))?
        }
        Err(_) => {
            // Decode/shaping/storage calls cannot be interrupted mid-call. Drain before
            // accepting another job; the deadline is not a hard wall-clock kill bound.
            let _ = task.await;
            return Err(timeout());
        }
    };
    check_deadline(deadline)?; // never publish a late result, including a late error
    result
}

#[derive(Debug)]
pub(super) struct Computed {
    pub value: Value,
    pub bytes: Arc<super::resource_bytes::ResourceBytes>,
}

fn compute_sync(store: &Storage, input: Value, control: Control) -> Result<Computed> {
    let started = Instant::now();
    control.check()?;
    let input: LayoutInput = serde_json::from_value(input)?;
    let resources = resources::load(store, &input, &control)?;
    control.check()?;
    let mut measurer = FontSetMeasurer::with_check(&resources.fonts, || {
        control.check().map_err(|error| LayoutError {
            code: error.0.code.clone(),
            message: error.0.message.clone(),
        })
    });
    let mut scene = opencanvas_layout::compute_with_measurer(&input, &mut measurer)
        .map_err(|error| ApiError::new(&error.code, &error.message))?;
    control.check()?;
    scene
        .diagnostics
        .extend(resources.diagnostics.iter().cloned());
    let result = json!({"status":"ready","scene":scene,"readiness":resources.readiness,
        "diagnostics":resources.diagnostics,
        "metrics":{"layoutPasses":1,"batchRPCs":0,"measuredItems":measurer.measured(),
            "cacheEntriesBefore":0,"elapsedMs":started.elapsed().as_secs_f64()*1000.0}});
    control.check()?;
    Ok(Computed {
        value: result,
        bytes: Arc::new(resources.bytes),
    })
}

pub(super) struct Control {
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}
impl Control {
    fn new(deadline: Instant) -> Self {
        Self {
            deadline,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
    pub(super) fn check(&self) -> Result<()> {
        check_deadline(self.deadline)?;
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(ApiError::new(
                "JOB_CANCELLED",
                "Native compute was cancelled",
            ));
        }
        Ok(())
    }
}
struct CancelOnDrop(Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

fn timeout() -> ApiError {
    ApiError::new("JOB_TIMEOUT", "Render job exceeded its deadline")
}
pub(super) fn check_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(timeout())
    } else {
        Ok(())
    }
}
