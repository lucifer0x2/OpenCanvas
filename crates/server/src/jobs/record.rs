//! Job record transitions out of `pending`: a finished run, or a queued prerender that left the
//! queue without running. Every transition wakes `wait`.
use super::{queue::Dropped, Jobs, Table};
use crate::error::{ApiError, Result};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::broadcast;

/// Error code of a queued prerender that never ran; see `queue::Queue::push_background`.
pub const SUPERSEDED: &str = "SUPERSEDED";

impl Table {
    /// Marks dropped prerenders failed with retryable `SUPERSEDED`; lookups never reuse them.
    pub(super) fn supersede(&mut self, dropped: Vec<Dropped>) {
        for Dropped { id, by } in dropped {
            let Some(entry) = self.entries.iter_mut().find(|e| e.value["jobId"] == id) else {
                continue;
            };
            let message = if by.is_some() {
                "A newer commit's prerender of this frame replaced this queued job"
            } else {
                "Displaced from the prerender queue by newer commits; request the scene with render_frame"
            };
            entry.value["status"] = json!("failed");
            entry.value["error"] = ApiError::new(SUPERSEDED, message).value()["error"].clone();
            if let Some(by) = by {
                entry.value["supersededBy"] = json!(by);
            }
        }
    }
}

impl Jobs {
    pub(super) async fn finish(
        &self,
        id: String,
        result: Result<(Value, Result<Value>)>,
        elapsed: Duration,
        events: &broadcast::Sender<Value>,
    ) {
        let mut table = self.table.lock().await;
        table.pace.record(elapsed);
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
