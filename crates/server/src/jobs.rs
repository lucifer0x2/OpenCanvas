mod input;
mod pace;
mod png;
mod queue;
mod record;
mod runner;
mod wait;
use crate::{
    error::{ApiError, Result},
    state::Config,
};
use opencanvas_storage::Snapshot;
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
    pub fn start(config: Arc<Config>, events: broadcast::Sender<Value>) -> Arc<Self> {
        let (doorbell, rings) = mpsc::channel(1);
        let jobs = Arc::new(Self {
            table: Mutex::new(Table::default()),
            doorbell,
            config: config.clone(),
            done: watch::channel(0).0,
        });
        runner::spawn(Arc::downgrade(&jobs), rings, config, events);
        jobs
    }
    pub async fn enqueue(&self, snapshot: Snapshot, request: SceneRequest) -> Result<Value> {
        self.enqueue_inner(snapshot, request, true, Priority::Direct)
            .await
    }
    /// Context reads retain failures for diagnosis. Explicit render_frame retries them.
    pub(crate) async fn for_context(
        &self,
        snapshot: Snapshot,
        request: SceneRequest,
    ) -> Result<Value> {
        self.enqueue_inner(snapshot, request, false, Priority::Direct)
            .await
    }
    /// A commit prerender: queued behind requested renders, superseded by a newer prerender of
    /// the same frame, and never rejected for a full requested tier.
    pub(crate) async fn prerender(
        &self,
        snapshot: Snapshot,
        request: SceneRequest,
    ) -> Result<Value> {
        self.enqueue_inner(snapshot, request, true, Priority::Background)
            .await
    }
    async fn enqueue_inner(
        &self,
        snapshot: Snapshot,
        request: SceneRequest,
        retry: bool,
        priority: Priority,
    ) -> Result<Value> {
        input::check_request(&snapshot, &request)?;
        let input = input::make_input(&snapshot, &request, &self.config.base_url)?;
        let key = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(
                &json!({"input":input,"scale":request.scale,"pipeline":self.config.pipeline_identity})
            )?)
        );
        let mut guard = self.table.lock().await;
        let table = &mut *guard;
        if let Some(entry) = table.entries.iter().rev().find(|e| {
            e.key == key
                && e.value["error"]["code"] != record::SUPERSEDED
                && (!retry || (e.value["status"] != "failed" && e.value.get("error").is_none()))
        }) {
            let value = entry.value.clone();
            if priority == Priority::Direct && value["status"] == "pending" {
                table
                    .queue
                    .promote(value["jobId"].as_str().unwrap_or_default());
            }
            return Ok(value);
        }
        while table.entries.len() >= 32 {
            if let Some(index) = table
                .entries
                .iter()
                .position(|e| e.value["status"] != "pending")
            {
                table.entries.remove(index);
            } else {
                return Err(table.pace.queue_full("Render job capacity reached"));
            }
        }
        let id = format!("job_{}", uuid::Uuid::new_v4().simple());
        let value = json!({"jobId":id,"status":"pending","projectId":snapshot.project_id,"revision":snapshot.revision,"frameId":request.frame_id,"sceneKey":key});
        let work = Work {
            id,
            key: key.clone(),
            input,
            scale: request.scale,
            frame: (snapshot.project_id.clone(), request.frame_id.clone()),
        };
        let mut dropped = Vec::new();
        match priority {
            Priority::Direct => {
                if !table.queue.push_direct(work) {
                    return Err(table.pace.queue_full("Render queue is full; retry later"));
                }
            }
            Priority::Background => dropped = table.queue.push_background(work),
        }
        table.entries.push_back(Entry {
            key,
            value: value.clone(),
            png: None,
        });
        if !dropped.is_empty() {
            table.supersede(dropped);
            self.done.send_modify(|n| *n = n.wrapping_add(1));
        }
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
