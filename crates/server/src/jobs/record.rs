//! Job record transitions out of `pending` when a run finishes. Every transition wakes `wait`.
use super::Jobs;
use crate::error::Result;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::broadcast;

impl Jobs {
    pub(super) async fn finish(
        &self,
        id: String,
        result: Result<(Value, Result<Value>)>,
        elapsed: Duration,
        events: &broadcast::Sender<Value>,
    ) {
        let mut table = self.table.lock().await;
        if result.is_ok() {
            table.pace.record(elapsed);
        }
        let Some(entry) = table.entries.iter_mut().find(|e| e.value["jobId"] == id) else {
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
                        entry.png = super::png::decode(&png).ok();
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
