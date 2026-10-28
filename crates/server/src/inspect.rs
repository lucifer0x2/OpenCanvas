//! `inspect_layout`: measured bounds and text lines of one render job's scene, paged.
use crate::{
    error::{ApiError, Result},
    reads::pagination,
    state::AppState,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;

const MAX_NODES: usize = 200;
const FIELDS: [&str; 2] = ["bounds", "lines"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    project_id: String,
    job_id: String,
    scene_key: String,
    node_ids: Option<Vec<String>>,
    fields: Option<Vec<String>>,
    limit: Option<usize>,
    /// Omitted means no byte cap, the v1 behaviour; the schema bounds an explicit value.
    budget: Option<usize>,
    cursor: Option<String>,
}

pub(crate) async fn inspect(state: &AppState, args: Value) -> Result<Value> {
    let req: Request = serde_json::from_value(args)?;
    let job = state.jobs.get(&req.job_id).await?;
    if job["projectId"] != req.project_id.as_str() || job["sceneKey"] != req.scene_key.as_str() {
        return Err(ApiError::invalid("Job/scene scope mismatch"));
    }
    if req
        .node_ids
        .as_ref()
        .is_some_and(|ids| ids.len() > MAX_NODES)
    {
        return Err(ApiError::invalid("At most 200 node IDs"));
    }
    let limit = req.limit.unwrap_or(MAX_NODES);
    if limit == 0 || limit > MAX_NODES {
        return Err(ApiError::invalid("inspect_layout limit must be 1–200"));
    }
    let fields = fields(req.fields.as_deref())?;
    // The job and its scene are immutable, so the cursor binds the job, not a revision:
    // later commits do not invalidate it; eviction of the job ends the read.
    let identity = json!([
        "inspect_layout",
        req.project_id,
        req.job_id,
        req.scene_key,
        req.node_ids,
        fields,
        limit,
        req.budget
    ]);
    let (scope, offset) = pagination::scope_of(&identity, req.cursor.as_deref())?;
    let wanted: Option<HashSet<&str>> = req
        .node_ids
        .as_ref()
        .map(|ids| ids.iter().map(String::as_str).collect());
    let entries: Vec<_> = job["scene"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| {
            wanted
                .as_ref()
                .is_none_or(|ids| n["id"].as_str().is_some_and(|id| ids.contains(id)))
        })
        .map(|n| project(n, &fields))
        .collect();
    let response = json!({"schemaVersion":1,"projectId":job["projectId"],"revision":job["revision"],"sceneKey":job["sceneKey"],"status":job["status"],"nodes":[],"diagnostics":job["diagnostics"]});
    let entries = entries
        .into_iter()
        .map(|n| ("nodes".into(), n, 0))
        .collect();
    let budget = req.budget.unwrap_or(usize::MAX);
    let mut page = pagination::page(response, entries, offset, limit, budget, 0, &scope)?;
    // A complete read keeps the v1 shape: nextCursor appears only with truncated:true.
    if page["nextCursor"].is_null() {
        page.as_object_mut().unwrap().remove("nextCursor");
    }
    Ok(page)
}

/// Requested node fields in canonical order; omitted means both.
fn fields(requested: Option<&[String]>) -> Result<Vec<&'static str>> {
    let Some(requested) = requested else {
        return Ok(FIELDS.to_vec());
    };
    if requested.is_empty() || requested.iter().any(|f| !FIELDS.contains(&f.as_str())) {
        return Err(ApiError::invalid("fields must list bounds and/or lines"));
    }
    Ok(FIELDS
        .into_iter()
        .filter(|f| requested.iter().any(|r| r == f))
        .collect())
}

fn project(node: &Value, fields: &[&str]) -> Value {
    let mut out = json!({"id": node["id"]});
    for field in fields {
        out[*field] = node[*field].clone();
    }
    out
}
