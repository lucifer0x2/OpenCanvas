mod export;
mod export_fonts;
mod export_svg;
mod input;
mod pace;
mod png;
mod queue;
mod record;
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
use opencanvas_storage::{Snapshot, Store};
use queue::{Priority, Queue, Work};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
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
    pub fn start(config: Arc<Config>, events: broadcast::Sender<Value>, store: Store) -> Arc<Self> {
        let (doorbell, rings) = mpsc::channel(1);
        let jobs = Arc::new(Self {
            table: Mutex::new(Table::default()),
            doorbell,
            config: config.clone(),
            done: watch::channel(0).0,
        });
        runner::spawn(Arc::downgrade(&jobs), rings, config, events, store);
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
        input::check_request(&snapshot, &request)?;
        let input = input::make_input(&snapshot, &request, &self.config.base_url)?;
        let text_overflow_intent = if snapshot.notes.is_empty() {
            Vec::new()
        } else {
            crate::text_diagnostics::overflow_ids(
                &snapshot,
                &serde_json::from_value::<Vec<opencanvas_storage::Node>>(input["nodes"].clone())?,
            )
        };
        let mut basis =
            json!({"input":input,"scale":request.scale,"pipeline":self.config.pipeline_identity});
        if let Some(spec) = &export {
            basis["export"] = json!(spec);
        }
        let key = format!("{:x}", Sha256::digest(serde_json::to_vec(&basis)?));
        let mut guard = self.table.lock().await;
        let table = &mut *guard;
        let direct = priority == Priority::Direct;
        if let Some(entry) = table.entries.iter_mut().rev().find(|e| {
            e.key == key
                && (!retry || (e.value["status"] != "failed" && e.value.get("error").is_none()))
        }) {
            entry.requested |= direct;
            let value = entry.value.clone();
            if direct && value["status"] == "pending" {
                table
                    .queue
                    .promote(value["jobId"].as_str().unwrap_or_default());
            }
            return Ok(value);
        }
        let id = format!("job_{}", uuid::Uuid::new_v4().simple());
        let mut value = json!({"jobId":id,"status":"pending","projectId":snapshot.project_id,"revision":snapshot.revision,"frameId":request.frame_id,"sceneKey":key});
        if let Some(spec) = &export {
            value["kind"] = json!("export");
            value["nodeId"] = json!(spec.node_id);
        }
        let work = Work {
            text_overflow_intent,
            id,
            key: key.clone(),
            input,
            scale: request.scale,
            frame: (snapshot.project_id.clone(), request.frame_id.clone()),
            export,
        };
        match priority {
            Priority::Direct => {
                table.make_room()?;
                if !table.queue.push_direct(work) {
                    // Every queued requested render plus the running job finish first.
                    let ahead = table.queue.direct_len() + 1;
                    return Err(table
                        .pace
                        .queue_full("Render queue is full; retry later", ahead));
                }
            }
            Priority::Background => {
                // Forget what this prerender replaces before counting records. At most
                // 1 + 8 + 8 + 8 (promoted) jobs are pending, so room is always found.
                let dropped = table.queue.push_background(work);
                table.forget(&dropped);
                table.make_room()?;
            }
        }
        table.entries.push_back(Entry {
            key,
            value: value.clone(),
            png: None,
            requested: direct,
            files: Vec::new(),
        });
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
