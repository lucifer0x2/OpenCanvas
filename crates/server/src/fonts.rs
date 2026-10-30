//! Project font inventory, exact-byte downloads and the existing import transaction.
use crate::{
    domain,
    error::{ApiError, Result},
    state::AppState,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue},
    response::Response,
    Json,
};
use opencanvas_storage::Snapshot;
use serde::Deserialize;
use serde_json::{json, Value};

pub(crate) fn metadata(snapshot: &Snapshot, id: &str, base: &str) -> Result<Value> {
    let font = snapshot
        .fonts
        .iter()
        .find(|font| font.id == id)
        .ok_or_else(|| ApiError::new("FONT_NOT_FOUND", "Font does not exist"))?;
    let mut value = serde_json::to_value(font)?;
    let (mime, size, source) = if let Some(asset) = &font.asset_id {
        let asset = snapshot
            .assets
            .iter()
            .find(|a| &a.id == asset)
            .ok_or_else(|| ApiError::new("FONT_MISSING", "Font asset is missing"))?;
        (asset.media_type.as_str(), asset.byte_size, "project")
    } else if font.bundled_id.as_deref() == Some("font_bundled") {
        (
            "font/ttf",
            include_bytes!("../../../renderer/fonts/NotoSans.ttf").len() as u64,
            "bundled",
        )
    } else {
        return Err(ApiError::new("FONT_MISSING", "Font binary is unavailable"));
    };
    value["mediaType"] = json!(mime);
    value["byteSize"] = json!(size);
    value["source"] = json!(source);
    value["downloadUrl"] = json!(format!(
        "{base}/api/v1/projects/{}/fonts/{id}/download",
        snapshot.project_id
    ));
    Ok(value)
}

pub async fn list(State(s): State<AppState>, Path(project): Path<String>) -> Result<Json<Value>> {
    let snapshot = s.snapshot(project).await?;
    let fonts = snapshot
        .fonts
        .iter()
        .map(|f| metadata(&snapshot, &f.id, &s.config.base_url))
        .collect::<Result<Vec<_>>>()?;
    let value = json!({"projectId":snapshot.project_id,"revision":snapshot.revision,"fonts":fonts});
    if serde_json::to_vec(&value)?.len() > 8 * 1024 * 1024 {
        return Err(ApiError::new(
            "CONTEXT_TOO_LARGE",
            "Font inventory exceeds the snapshot budget; use list_fonts pagination",
        ));
    }
    Ok(Json(value))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Import {
    expected_revision: u64,
    idempotency_key: String,
    media_type: String,
    base64: String,
    font: opencanvas_storage::FontRegistration,
}

pub async fn import(
    State(s): State<AppState>,
    Path(project): Path<String>,
    body: std::result::Result<Json<Import>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>> {
    let body = body
        .map_err(|_| ApiError::invalid("Invalid font import body"))?
        .0;
    // Resolve slugs too, then forward the canonical ID to the domain transaction.
    let snapshot = s.snapshot(project).await?;
    let result = domain::mutate(
        &s,
        "import_asset",
        json!({"projectId":snapshot.project_id,
        "expectedRevision":body.expected_revision,"idempotencyKey":body.idempotency_key,
        "mediaType":body.media_type,"base64":body.base64,"font":body.font}),
    )
    .await?;
    Ok(Json(result))
}

pub async fn download(
    State(s): State<AppState>,
    Path((project, id)): Path<(String, String)>,
) -> Result<Response> {
    let (font, mime, bytes) = s.project(project, move |p| p.read_font(&id)).await?;
    let extension = match mime.as_str() {
        "font/ttf" => "ttf",
        "font/otf" => "otf",
        "font/woff2" => "woff2",
        _ => return Err(ApiError::new("FONT_INVALID", "Unsupported font type")),
    };
    // IDs are validated ASCII identifiers; strip again before building an HTTP header.
    let safe: String = font
        .id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_str(&mime).unwrap());
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{safe}.{extension}\"")).unwrap(),
    );
    Ok(response)
}
