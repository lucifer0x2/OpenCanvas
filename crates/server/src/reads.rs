mod pagination;
mod tree;
use crate::{error::Result, state::AppState};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ReadRequest {
    project_id: String,
    node_id: Option<String>,
    frame_id: Option<String>,
    depth: Option<usize>,
    limit: Option<usize>,
    budget: Option<usize>,
    ids: Option<Vec<String>>,
    properties: Option<Vec<String>>,
    viewport: Option<Value>,
    theme: Option<String>,
    cursor: Option<String>,
    revision: Option<u64>,
}
pub async fn read(state: &AppState, name: &str, args: Value) -> Result<Value> {
    let req: ReadRequest = serde_json::from_value(args)?;
    let s = state.snapshot(req.project_id.clone()).await?;
    let (scope, offset) = pagination::scope(&s, name, &req)?;
    if matches!(name, "get_outline" | "get_subtree" | "get_design_context") {
        return tree::read(&s, name, req, scope, offset);
    }
    let value = serde_json::to_value(&s)?;
    let field = name.trim_start_matches("list_");
    let entries: Vec<_> = value[field]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| {
            req.ids
                .as_ref()
                .is_none_or(|ids| ids.iter().any(|id| v["id"] == *id))
        })
        .cloned()
        .map(sanitize_asset)
        .map(|v| ("items".into(), v, 0))
        .collect();
    let response =
        json!({"schemaVersion":1,"projectId":s.project_id,"revision":s.revision,"items":[]});
    pagination::page(
        response,
        entries,
        offset,
        req.limit.unwrap_or(100),
        65536,
        8,
        &scope,
    )
}
fn sanitize_asset(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.remove("relativeKey");
    }
    value
}
