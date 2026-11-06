mod admission;
mod export;
mod export_fonts;
mod export_resources;
mod export_scope;
mod export_svg;
mod input;
mod native;
mod pace;
mod painter;
mod png;
mod prepared;
mod queue;
mod record;
mod resource_bytes;
mod retain;
mod runner;
mod states;
mod subtree;
mod wait;
use crate::{
    error::{ApiError, Result},
    state::Config,
};
pub use export::{ExportSetting, ExportSpec};
use opencanvas_storage::{Snapshot, Storage};
use queue::{Priority, Queue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{broadcast, mpsc, watch, Mutex};
pub use wait::MAX_WAIT_MS;

fn default_name() -> String {
    "default".into()
}
fn one() -> f64 {
    1.0
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneRequest {
    pub frame_id: String,
    pub revision: u64,
    pub width: f64,
    pub height: f64,
    #[serde(default = "default_name")]
    pub theme: String,
    #[serde(default = "default_name")]
    pub state: String,
    #[serde(default = "one")]
    pub scale: f64,
}
struct Entry {
    key: String,
    value: Value,
    png: Option<Vec<u8>>,
    /// A requested render created or reused this job, so a caller may still read it by ID.
    requested: bool,
    files: Vec<export::StoredFile>,
}
/// Job records and the queue share one lock, so a queued job always has its record.
#[derive(Default)]
struct Table {
    entries: VecDeque<Entry>,
    queue: Queue,
    pace: pace::Pace,
}
pub struct Jobs {
    table: Mutex<Table>,
    /// Wakes the render loop; dropping it with `Jobs` ends the loop.
    doorbell: mpsc::Sender<()>,
    config: Arc<Config>,
    /// Bumped after every job leaves `pending`; `wait` parks on it.
    done: watch::Sender<u64>,
}
impl Jobs {
    pub fn start(
        config: Arc<Config>,
        events: broadcast::Sender<Value>,
        store: impl Into<Storage>,
    ) -> Arc<Self> {
        let (doorbell, rings) = mpsc::channel(1);
        let jobs = Arc::new(Self {
            table: Mutex::new(Table::default()),
            doorbell,
            config: config.clone(),
            done: watch::channel(0).0,
        });
        runner::spawn(Arc::downgrade(&jobs), rings, config, events, store.into());
        jobs
    }
    pub async fn enqueue(&self, snapshot: Snapshot, request: SceneRequest) -> Result<Value> {
        self.enqueue_inner(snapshot, request, true, Priority::Direct, None)
            .await
    }
    /// Context reads retain failures for diagnosis. Explicit render_frame retries them.
    pub(crate) async fn for_context(
        &self,
        snapshot: Snapshot,
        request: SceneRequest,
    ) -> Result<Value> {
        self.enqueue_inner(snapshot, request, false, Priority::Direct, None)
            .await
    }
    /// A commit prerender: queued behind requested renders, superseded by a newer prerender of
    /// the same frame, and never rejected for a full requested tier.
    pub(crate) async fn prerender(
        &self,
        snapshot: Snapshot,
        request: SceneRequest,
    ) -> Result<Value> {
        self.enqueue_inner(snapshot, request, true, Priority::Background, None)
            .await
    }
    /// A requested export of one node, laid out in its root frame (`request`) at scale 1.
    pub(crate) async fn export(
        &self,
        snapshot: Snapshot,
        request: SceneRequest,
        spec: ExportSpec,
    ) -> Result<Value> {
        self.enqueue_inner(snapshot, request, true, Priority::Direct, Some(spec))
            .await
    }
    async fn enqueue_inner(
        &self,
        snapshot: Snapshot,
        request: SceneRequest,
        retry: bool,
        priority: Priority,
        export: Option<ExportSpec>,
    ) -> Result<Value> {
        let prepared = prepared::Prepared::new(&snapshot, request, &self.config, export)?;
        let mut guard = self.table.lock().await;
        let value = guard.admit(prepared, retry, priority)?;
        drop(guard);
        // Full means a ring is already pending; the loop drains the queue on it.
        let _ = self.doorbell.try_send(());
        Ok(value)
    }
    pub async fn get(&self, id: &str) -> Result<Value> {
        self.table
            .lock()
            .await
            .entries
            .iter()
            .find(|e| e.value["jobId"] == id)
            .map(|e| e.value.clone())
            .ok_or_else(|| ApiError::new("JOB_NOT_FOUND", "Render job is unavailable"))
    }
    pub(crate) async fn project_id(&self, id: &str) -> Result<String> {
        self.table
            .lock()
            .await
            .entries
            .iter()
            .find(|entry| entry.value["jobId"] == id)
            .and_then(|entry| entry.value["projectId"].as_str())
            .map(str::to_owned)
            .ok_or_else(|| ApiError::new("JOB_NOT_FOUND", "Render job is unavailable"))
    }
    pub async fn image(&self, id: &str) -> Result<Vec<u8>> {
        let table = self.table.lock().await;
        let e = table
            .entries
            .iter()
            .find(|e| e.value["jobId"] == id)
            .ok_or_else(|| ApiError::new("JOB_NOT_FOUND", "Render job is unavailable"))?;
        e.png.clone().ok_or_else(|| {
            ApiError::new(
                "IMAGE_NOT_READY",
                "Strict PNG export is pending or failed; inspect job error",
            )
        })
    }
}
