//! `export_nodes`: image files of nodes. Each node is one export job (its root frame computed
//! once, one PNG per setting); the call waits for the jobs up to `waitMs` and reports each node
//! on its own, so one bad node never fails the others.
mod download;
mod target;
use crate::{
    error::{ApiError, Result},
    jobs::MAX_WAIT_MS,
    state::AppState,
};
pub(crate) use download::file;
use opencanvas_storage::{check_export_settings, resolved_nodes_for_theme};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::Instant;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    project_id: String,
    node_ids: Vec<String>,
    settings: Option<Value>,
    theme: Option<String>,
    revision: Option<u64>,
    wait_ms: Option<u64>,
}

pub(crate) async fn export_nodes(state: &AppState, args: Value) -> Result<Value> {
    let req: Request = serde_json::from_value(args)?;
    if let Some(settings) = &req.settings {
        check_export_settings(settings, "settings")?;
    }
    let snapshot = state.snapshot(req.project_id).await?;
    if req.revision.is_some_and(|r| r != snapshot.revision) {
        return Err(ApiError::new(
            "REVISION_UNAVAILABLE",
            "Requested authored revision is unavailable",
        ));
    }
    let theme = req.theme.as_deref().unwrap_or("default");
    let resolved = resolved_nodes_for_theme(&snapshot, theme)?;
    let nodes = target::Nodes::new(&snapshot, &resolved);
    let mut jobs = Vec::with_capacity(req.node_ids.len());
    for node_id in &req.node_ids {
        let job = match target::resolve(&nodes, &snapshot, node_id, theme, req.settings.as_ref()) {
            Ok((request, spec)) => state.jobs.export(snapshot.clone(), request, spec).await,
            Err(error) => Err(error),
        };
        jobs.push(job);
    }
    let wait = Duration::from_millis(req.wait_ms.unwrap_or(MAX_WAIT_MS).min(MAX_WAIT_MS));
    let deadline = Instant::now() + wait;
    let mut exports = Vec::with_capacity(jobs.len());
    for (node_id, job) in req.node_ids.iter().zip(jobs) {
        let job = match job {
            Ok(job) if job["status"] == "pending" => {
                let id = job["jobId"].as_str().unwrap_or_default();
                let left = deadline.saturating_duration_since(Instant::now());
                state.jobs.wait(id, left, state.shutting_down()).await
            }
            other => other,
        };
        exports.push(item(node_id, job));
    }
    Ok(json!({"projectId":snapshot.project_id,"revision":snapshot.revision,"exports":exports}))
}

/// One node's entry: its job and files, or the error that stopped it.
fn item(node_id: &str, job: Result<Value>) -> Value {
    match job {
        Ok(job) => {
            let mut item = json!({"nodeId":node_id,"jobId":job["jobId"],"status":job["status"]});
            for key in ["files", "error"] {
                if let Some(value) = job.get(key) {
                    item[key] = value.clone();
                }
            }
            item
        }
        Err(error) => {
            json!({"nodeId":node_id,"jobId":null,"status":"failed","error":error.value()["error"]})
        }
    }
}
