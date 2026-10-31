//! Viewer export requests share the MCP schema, validation and immutable export pipeline.
use crate::{
    error::{ApiError, Result},
    schemas,
    state::AppState,
    viewer_origin::{self, Origin},
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};

pub(crate) async fn create(
    State(state): State<AppState>,
    Path(project): Path<String>,
    origin: Origin,
    body: std::result::Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>> {
    let mut args = body
        .map_err(|_| ApiError::invalid("Invalid export JSON body"))?
        .0;
    let object = args
        .as_object_mut()
        .ok_or_else(|| ApiError::invalid("Expected an export object"))?;
    // The URL is authoritative; reject an accidental or malicious alternate scope.
    if object.contains_key("projectId") {
        return Err(ApiError::invalid("projectId belongs in the URL"));
    }
    object.insert("projectId".into(), json!(project));
    schemas::validate("export_nodes", &args)?;
    Ok(Json(viewer_origin::exports(
        super::export_nodes(&state, args).await?,
        &state.config.base_url,
        origin,
    )))
}
