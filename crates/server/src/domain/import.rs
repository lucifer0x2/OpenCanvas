//! `import_asset` takes its bytes inline as base64 or from a file in the project's inbox.
//! Both reach storage as the same decoded bytes, so replay and the asset hash ignore the source.
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use opencanvas_storage::{FontRegistration, ImportAssetRequest};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportRequest {
    project_id: String,
    expected_revision: u64,
    idempotency_key: String,
    media_type: String,
    base64: Option<String>,
    path: Option<String>,
    font: Option<FontRegistration>,
}

enum Source {
    Bytes(Vec<u8>),
    Inbox(String),
}

pub(super) async fn run(state: &AppState, args: Value) -> Result<Value> {
    let req: ImportRequest = serde_json::from_value(args)?;
    let source = match (req.base64, req.path) {
        (Some(base64), None) => Source::Bytes(decode(&base64)?),
        (None, Some(path)) => Source::Inbox(path),
        _ => return Err(ApiError::invalid("Send exactly one of path or base64")),
    };
    let mut import = ImportAssetRequest {
        project_id: req.project_id,
        expected_revision: req.expected_revision,
        idempotency_key: req.idempotency_key,
        media_type: req.media_type,
        bytes: Vec::new(),
        font: req.font,
    };
    let (id, expected) = (import.project_id.clone(), import.expected_revision);
    let result = state
        .project(id.clone(), move |p| {
            import.bytes = match source {
                Source::Bytes(bytes) => bytes,
                Source::Inbox(path) => p.read_inbox_asset(&path)?,
            };
            p.import_asset(import)
        })
        .await;
    Ok(with_commit_fields(serde_json::to_value(
        conflict::checked(state, &id, expected, result).await?,
    )?))
}

fn decode(base64: &str) -> Result<Vec<u8>> {
    if base64.len() > 13_981_016 {
        return Err(ApiError::invalid("Asset exceeds 10 MiB"));
    }
    let bytes = STANDARD
        .decode(base64)
        .map_err(|_| ApiError::invalid("Invalid base64 asset"))?;
    if bytes.len() > opencanvas_storage::MAX_ASSET_BYTES {
        return Err(ApiError::invalid("Asset exceeds 10 MiB"));
    }
    Ok(bytes)
}

/// import_asset answers like apply_batch, with the commit fields at the top level; `result`
/// still carries them for clients written against the earlier nested shape.
fn with_commit_fields(mut value: Value) -> Value {
    let commit = value.get("result").and_then(Value::as_object).cloned();
    if let (Some(commit), Some(top)) = (commit, value.as_object_mut()) {
        for (key, field) in commit {
            top.entry(key).or_insert(field);
        }
    }
    value
}
