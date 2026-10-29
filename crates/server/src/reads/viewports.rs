use super::{index::TreeIndex, ReadRequest};
use crate::{
    error::{ApiError, Result},
    jobs::SceneRequest,
    state::AppState,
};
use opencanvas_storage::{NodeKind, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Viewport {
    width: f64,
    height: f64,
}

pub(super) struct Computed {
    pub metadata: Value,
    pub bounds: BTreeMap<String, Vec<Value>>,
    pub reflow: Option<bool>,
    pub lines: BTreeMap<String, Vec<Value>>,
}

/// Enqueue/read the existing bounded jobs; never wait on Chromium in a context read.
pub(super) async fn compute(
    state: &AppState,
    snapshot: &Snapshot,
    req: &ReadRequest,
) -> Result<Option<Computed>> {
    let Some(viewports) = &req.viewports else {
        return Ok(None);
    };
    if req.viewport.is_some() || req.node_id.is_some() {
        return Err(ApiError::invalid(
            "viewports requires frameId and cannot be combined with viewport or nodeId",
        ));
    }
    let frame = req
        .frame_id
        .as_deref()
        .ok_or_else(|| ApiError::invalid("frameId is required"))?;
    if !snapshot
        .nodes
        .iter()
        .any(|n| n.id == frame && n.kind == NodeKind::Frame)
    {
        return Err(ApiError::invalid("frameId must identify a frame"));
    }
    validate(viewports)?;
    let expanded = opencanvas_storage::instances::expand(&snapshot.nodes)?;
    let ids: HashSet<String> = TreeIndex::new(&expanded)
        .preorder(Some(frame))
        .into_iter()
        .map(|(n, _)| n.id.clone())
        .collect();
    let mut bounds: BTreeMap<_, _> = ids.iter().map(|id| (id.clone(), Vec::new())).collect();
    let mut metadata = Vec::new();
    let mut lines: BTreeMap<String, Vec<Value>> =
        ids.iter().map(|id| (id.clone(), Vec::new())).collect();
    let mut pending = false;
    let mut failed = false;
    for viewport in viewports {
        let request = SceneRequest {
            frame_id: frame.into(),
            revision: snapshot.revision,
            width: viewport.width,
            height: viewport.height,
            theme: req.theme.clone().unwrap_or_else(|| "default".into()),
            state: "default".into(),
            scale: 1.0,
        };
        let job = match state.jobs.for_context(snapshot.clone(), request).await {
            Ok(job) => job,
            Err(e) if e.0.code == "QUEUE_FULL" => {
                json!({"status":"failed","error":e.value()["error"]})
            }
            Err(e) => return Err(e),
        };
        let mut item = json!({"viewport":viewport,"jobId":job["jobId"],"sceneKey":job["sceneKey"],"status":job["status"]});
        if let Some(error) = job.get("error") {
            item["status"] = json!("failed");
            item["error"] = error.clone();
        }
        let scene = scene_bounds(&job, &ids);
        if item["status"] == "ready" && scene.is_none() {
            item["status"] = json!("failed");
            item["error"] = json!({"code":"INCOMPLETE_SCENE","message":"Scene does not contain complete finite bounds for the frame scope","retryable":true});
        }
        pending |= item["status"] == "pending";
        failed |= item["status"] == "failed";
        for (id, values) in &mut bounds {
            values.push(if item["status"] == "ready" {
                scene.as_ref().unwrap()[id].clone()
            } else {
                Value::Null
            });
        }
        let scene_lines: BTreeMap<_, _> = job["scene"]["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|n| Some((n["id"].as_str()?, n["lines"].clone())))
            .collect();
        for (id, values) in &mut lines {
            values.push(if item["status"] == "ready" {
                scene_lines.get(id.as_str()).cloned().unwrap_or(Value::Null)
            } else {
                Value::Null
            });
        }
        metadata.push(item);
    }
    // Compare the entire descendant scope, never just the current page or filtered props.
    let reflow = if failed || pending || viewports.len() < 2 {
        None
    } else {
        Some(
            bounds
                .iter()
                .filter(|(id, _)| id.as_str() != frame)
                .any(|(_, values)| values[1..].iter().any(|v| v != &values[0])),
        )
    };
    Ok(Some(Computed {
        metadata: json!({"status":if failed {"failed"} else if pending {"pending"} else {"ready"},
            "coordinateSpace":"frame-local","viewports":metadata,
            "comparisonScope":"all-descendants-excluding-root","comparedNodes":ids.len()-1}),
        bounds,
        reflow,
        lines,
    }))
}

fn validate(viewports: &[Viewport]) -> Result<()> {
    let pixels: f64 = viewports.iter().map(|v| v.width * v.height).sum();
    if viewports.is_empty()
        || viewports.len() > 4
        || !pixels.is_finite()
        || pixels > 16_000_000.0
        || viewports.iter().any(|v| {
            !v.width.is_finite()
                || !v.height.is_finite()
                || v.width <= 0.0
                || v.height <= 0.0
                || v.width > 100_000.0
                || v.height > 100_000.0
        })
    {
        return Err(ApiError::invalid("viewports requires 1–4 positive viewports, dimensions at most 100000 and total area at most 16 million pixels"));
    }
    Ok(())
}

/// The frame and every descendant, in one indexed walk (was a fixed-point rescan of all nodes).
fn scene_bounds(job: &Value, ids: &HashSet<String>) -> Option<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    for node in job["scene"]["nodes"].as_array()? {
        let id = node["id"].as_str()?;
        let bounds = &node["bounds"];
        if !ids.contains(id)
            || out.contains_key(id)
            || ["x", "y", "width", "height"]
                .iter()
                .any(|key| bounds[key].as_f64().is_none_or(|v| !v.is_finite()))
        {
            return None;
        }
        out.insert(id.into(), bounds.clone());
    }
    (out.len() == ids.len()).then_some(out)
}
