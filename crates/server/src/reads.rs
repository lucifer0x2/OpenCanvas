mod animated;
pub(crate) mod changes;
mod compact;
mod diagnostics;
mod find;
mod index;
mod navigation;
mod notes;
pub(crate) mod pagination;
pub(crate) mod targets;
#[cfg(test)]
mod traversal_tests;
mod tree;
mod viewports;
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
    viewports: Option<Vec<viewports::Viewport>>,
    theme: Option<String>,
    cursor: Option<String>,
    revision: Option<u64>,
    // find_nodes only. Skipped when absent so older tools keep their cursor scope hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kinds: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name_contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text_contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent_id: Option<String>,
    // get_design_context only; false is normalized to absent so the default scope hash holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compact: Option<bool>,
    // get_design_context only; "open" (the default) is normalized to absent, like compact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    notes: Option<notes::Filter>,
}
pub async fn read(state: &AppState, name: &str, args: Value) -> Result<Value> {
    let mut req: ReadRequest = serde_json::from_value(args)?;
    req.compact = req.compact.filter(|on| *on);
    req.notes = req.notes.filter(|f| *f != notes::Filter::Open);
    let s = state.snapshot(req.project_id.clone()).await?;
    if name == "find_nodes" {
        return find::read(&s, req);
    }
    let (scope, offset) = pagination::scope(&s, name, &req)?;
    if matches!(name, "get_outline" | "get_subtree" | "get_design_context") {
        let computed = if name == "get_design_context" {
            viewports::compute(state, &s, &req).await?
        } else {
            None
        };
        return tree::read(&s, name, req, scope, offset, computed);
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
        .map(|v| ("items".into(), v))
        .collect();
    let response =
        json!({"schemaVersion":1,"projectId":s.project_id,"revision":s.revision,"items":[]});
    pagination::page(
        response,
        entries,
        offset,
        req.limit.unwrap_or(100),
        65536,
        &scope,
    )
}
fn sanitize_asset(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.remove("relativeKey");
    }
    value
}
