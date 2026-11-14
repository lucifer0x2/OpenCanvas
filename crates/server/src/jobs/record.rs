//! Job record transitions out of `pending` when a run finishes. Every transition wakes `wait`.
use super::{export::Rendered, export_store::ExportStore, Entry, Jobs};
use crate::error::Result;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;

impl Jobs {
    pub(super) async fn finish(
        &self,
        id: String,
        result: Result<(Value, Result<Rendered>)>,
        elapsed: Duration,
        deadline: Instant,
        events: &broadcast::Sender<Value>,
    ) {
        let mut guard = self.table.lock().await;
        let table = &mut *guard;
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
            Ok((computed, rendered)) if entry.value["kind"] == "export" => {
                super::export::finish(entry, &self.config.base_url, &computed, rendered);
            }
            Ok((computed, rendered)) => {
                entry.value["status"] = json!("ready");
                entry.value["scene"] = computed["scene"].clone();
                entry.value["provenance"] = computed["scene"]["provenance"].clone();
                entry.value["diagnostics"] = computed["scene"]["diagnostics"].clone();
                entry.value["readiness"] = computed["readiness"].clone();
                match rendered {
                    Ok(Rendered::Frame(png)) => {
                        entry.png = Some(png.bytes);
                        let png = png.value;
                        entry.value["imageUrl"] =
                            json!(format!("{}/api/v1/jobs/{id}/image", self.config.base_url));
                        entry.value["image"] = json!({"uri":entry.value["imageUrl"],"mimeType":"image/png","width":png["width"],"height":png["height"],"provenance":png["provenance"]});
                        for field in [
                            "browserVersion",
                            "runtimeVersion",
                            "engine",
                            "engineVersion",
                        ] {
                            if png["provenance"][field].is_null() {
                                continue;
                            }
                            entry.value["scene"]["provenance"][field] =
                                png["provenance"][field].clone();
                            entry.value["provenance"][field] = png["provenance"][field].clone();
                        }
                    }
                    Ok(Rendered::Export(_)) => {}
                    Err(error) => {
                        entry.value["error"] = json!(error.0);
                    }
                }
            }
        }
        // The table lock is the publication boundary. Guard AFTER storage/metadata
        // conversion too: a completed paint or SVG can expire while awaiting this lock.
        if self.config.render_backend == crate::RenderBackend::Rust {
            reject_expired(entry, deadline);
        }
        let evicted = publish_files(entry, &mut table.exports);
        if entry.value.get("scene").is_some() {
            let _=events.send(json!({"event":"scene_ready","projectId":entry.value["projectId"],"revision":entry.value["revision"],"sceneKey":entry.value["sceneKey"],"jobId":id}));
        }
        table.forget(&evicted);
        self.done.send_modify(|n| *n = n.wrapping_add(1));
    }
}

/// Moves a ready export's files into the export store, out of reach of record eviction, and
/// returns the jobs whose files the byte budget evicted.
fn publish_files(entry: &mut Entry, store: &mut ExportStore) -> Vec<String> {
    let files = std::mem::take(&mut entry.files);
    if entry.value["kind"] != "export" || entry.value["status"] != "ready" {
        return Vec::new();
    }
    let id = entry.value["jobId"].as_str().unwrap_or_default();
    let project = entry.value["projectId"].as_str().unwrap_or_default();
    let (expires_at, evicted) = store.insert(id, project, files, Instant::now());
    entry.value["expiresAt"] = json!(expires_at);
    evicted
}

fn reject_expired(entry: &mut Entry, deadline: Instant) {
    if entry.value["status"] != "ready" {
        return;
    }
    let Err(error) = super::native::check_deadline(deadline) else {
        return;
    };
    entry.png = None;
    entry.files.clear();
    let fields = entry.value.as_object_mut().unwrap();
    for key in ["imageUrl", "image", "files"] {
        fields.remove(key);
    }
    if fields.get("kind").is_some_and(|kind| kind == "export") {
        fields.insert("status".into(), json!("failed"));
    }
    fields.insert("error".into(), json!(error.0));
}
