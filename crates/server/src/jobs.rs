mod input;
mod png;
mod wait;
use crate::{
    error::{ApiError, Result},
    state::Config,
    worker::Worker,
};
use opencanvas_storage::Snapshot;
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
struct Work {
    id: String,
    key: String,
    input: Value,
    scale: f64,
}
pub struct Jobs {
    entries: Mutex<VecDeque<Entry>>,
    sender: mpsc::Sender<Work>,
    config: Arc<Config>,
    /// Bumped after every job leaves `pending`; `wait` parks on it.
    done: watch::Sender<u64>,
}
impl Jobs {
    pub fn start(config: Arc<Config>, events: broadcast::Sender<Value>) -> Arc<Self> {
        let (sender, mut receiver) = mpsc::channel::<Work>(8);
        let jobs = Arc::new(Self {
            entries: Mutex::new(VecDeque::new()),
            sender,
            config: config.clone(),
            done: watch::channel(0).0,
        });
        let weak = Arc::downgrade(&jobs);
        tokio::spawn(async move {
            let mut worker: Option<Worker> = None;
            while let Some(work) = receiver.recv().await {
                let result = tokio::time::timeout(config.job_timeout, async {
                    if worker.is_none() { worker = Some(Worker::start(&config)?); }
                    let w = worker.as_mut().unwrap();
                    let mut computed = w.call(json!({"op":"compute","input":work.input,"strict":false})).await?;
                    computed["scene"]["provenance"]["sourceSceneKey"] = json!(work.key);
                    let rendered = w.call(json!({"op":"render","scene":computed["scene"],"scale":work.scale,"strict":true})).await.and_then(|result| { png::decode(&result)?; Ok(result) });
                    Ok::<_,ApiError>((computed,rendered))
                }).await.unwrap_or_else(|_|Err(ApiError::new("JOB_TIMEOUT","Render job exceeded its deadline")));
                // Restart after any failure, including a strict export/resource failure.
                let unhealthy = result.as_ref().map_or(true, |(_, r)| r.is_err());
                if unhealthy {
                    if let Some(mut w) = worker.take() {
                        w.stop().await;
                    }
                }
                let Some(jobs) = weak.upgrade() else {
                    break;
                };
                jobs.finish(work.id, result, &events).await;
            }
            if let Some(mut w) = worker {
                w.stop().await;
            }
        });
        jobs
    }
    pub async fn enqueue(&self, snapshot: Snapshot, request: SceneRequest) -> Result<Value> {
        self.enqueue_inner(snapshot, request, true).await
    }
    /// Context reads retain failures for diagnosis. Explicit render_frame retries them.
    pub(crate) async fn for_context(
        &self,
        snapshot: Snapshot,
        request: SceneRequest,
    ) -> Result<Value> {
        self.enqueue_inner(snapshot, request, false).await
    }
    async fn enqueue_inner(
        &self,
        snapshot: Snapshot,
        request: SceneRequest,
        retry: bool,
    ) -> Result<Value> {
        if snapshot.revision != request.revision {
            return Err(ApiError::new(
                "REVISION_UNAVAILABLE",
                "Requested authored revision is unavailable",
            ));
        }
        if !request.width.is_finite()
            || !request.height.is_finite()
            || request.width <= 0.0
            || request.height <= 0.0
            || request.width > 100000.0
            || request.height > 100000.0
            || !request.scale.is_finite()
            || request.scale <= 0.0
            || request.scale > 4.0
            || request.width * request.height * request.scale * request.scale > 16_000_000.0
        {
            return Err(ApiError::invalid(
                "Viewport or scale exceeds the render budget",
            ));
        }
        if request.state != "default" {
            return Err(ApiError::invalid(
                "Only default interaction state is supported",
            ));
        }
        let input = input::make_input(&snapshot, &request, &self.config.base_url)?;
        let key = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(
                &json!({"input":input,"scale":request.scale,"pipeline":self.config.pipeline_identity})
            )?)
        );
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.iter().rev().find(|e| {
            e.key == key
                && (!retry || (e.value["status"] != "failed" && e.value.get("error").is_none()))
        }) {
            return Ok(entry.value.clone());
        }
        while entries.len() >= 32 {
            if let Some(index) = entries.iter().position(|e| e.value["status"] != "pending") {
                entries.remove(index);
            } else {
                return Err(ApiError::new("QUEUE_FULL", "Render job capacity reached"));
            }
        }
        let id = format!("job_{}", uuid::Uuid::new_v4().simple());
        let value = json!({"jobId":id,"status":"pending","projectId":snapshot.project_id,"revision":snapshot.revision,"frameId":request.frame_id,"sceneKey":key});
        self.sender
            .try_send(Work {
                id,
                key: key.clone(),
                input,
                scale: request.scale,
            })
            .map_err(|_| ApiError::new("QUEUE_FULL", "Render queue is full; retry later"))?;
        entries.push_back(Entry {
            key,
            value: value.clone(),
            png: None,
        });
        Ok(value)
    }
    pub async fn get(&self, id: &str) -> Result<Value> {
        self.entries
            .lock()
            .await
            .iter()
            .find(|e| e.value["jobId"] == id)
            .map(|e| e.value.clone())
            .ok_or_else(|| ApiError::new("JOB_NOT_FOUND", "Render job is unavailable"))
    }
    pub async fn image(&self, id: &str) -> Result<Vec<u8>> {
        let entries = self.entries.lock().await;
        let e = entries
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
    async fn finish(
        &self,
        id: String,
        result: Result<(Value, Result<Value>)>,
        events: &broadcast::Sender<Value>,
    ) {
        let mut entries = self.entries.lock().await;
        let Some(entry) = entries.iter_mut().find(|e| e.value["jobId"] == id) else {
            return;
        };
        match result {
            Err(error) => {
                entry.value["status"] = json!("failed");
                entry.value["error"] = json!(error.0);
            }
            Ok((computed, rendered)) => {
                entry.value["status"] = json!("ready");
                entry.value["scene"] = computed["scene"].clone();
                entry.value["provenance"] = computed["scene"]["provenance"].clone();
                entry.value["diagnostics"] = computed["scene"]["diagnostics"].clone();
                entry.value["readiness"] = computed["readiness"].clone();
                match rendered {
                    Ok(png) => {
                        entry.png = png::decode(&png).ok();
                        entry.value["imageUrl"] =
                            json!(format!("{}/api/v1/jobs/{id}/image", self.config.base_url));
                        entry.value["image"] = json!({"uri":entry.value["imageUrl"],"mimeType":"image/png","width":png["width"],"height":png["height"],"provenance":png["provenance"]});
                        for field in ["browserVersion", "runtimeVersion"] {
                            entry.value["scene"]["provenance"][field] =
                                png["provenance"][field].clone();
                            entry.value["provenance"][field] = png["provenance"][field].clone();
                        }
                    }
                    Err(error) => {
                        entry.value["error"] = json!(error.0);
                    }
                }
                let _=events.send(json!({"event":"scene_ready","projectId":entry.value["projectId"],"revision":entry.value["revision"],"sceneKey":entry.value["sceneKey"],"jobId":id}));
            }
        }
        self.done.send_modify(|n| *n = n.wrapping_add(1));
    }
}
