//! SVG files of an export job: built in Rust from the node's subtree scene, never by the worker.
//! Resources are strict like a PNG export: any failed font or asset, or an image whose stored
//! bytes no longer match the scene's manifest hash, fails the job instead of a partial SVG.
use super::export::{ExportSetting, StoredFile};
use crate::{
    error::{ApiError, Result},
    svg::{scene_to_svg, Image},
};
use opencanvas_storage::Store;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Where the job reads image bytes: the project's asset store.
pub struct AssetSource {
    pub store: Store,
    pub project_id: String,
}

fn not_ready(message: &str) -> ApiError {
    ApiError::new("RESOURCE_NOT_READY", message)
}

/// The images the subtree paints, read once per job.
pub async fn images(
    source: &AssetSource,
    scene: &Value,
    readiness: &Value,
) -> Result<HashMap<String, Image>> {
    let failed = readiness
        .as_array()
        .is_some_and(|items| items.iter().any(|r| r["status"] == "failed"));
    if failed {
        return Err(not_ready("Scene resources are not ready"));
    }
    let mut wanted: Vec<(String, String)> = Vec::new();
    for node in scene["nodes"].as_array().into_iter().flatten() {
        let Some(id) = node["assetId"].as_str().filter(|_| node["kind"] == "image") else {
            continue;
        };
        if wanted.iter().any(|(have, _)| have == id) {
            continue;
        }
        let hash = scene["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|a| a["id"] == id)
            .and_then(|a| a["hash"].as_str())
            .ok_or_else(|| not_ready("Image asset is not in the scene manifest"))?;
        wanted.push((id.into(), hash.into()));
    }
    if wanted.is_empty() {
        return Ok(HashMap::new());
    }
    let (store, project) = (source.store.clone(), source.project_id.clone());
    tokio::task::spawn_blocking(move || {
        let project = store
            .open_project(&project)
            .map_err(|_| not_ready("Project is unavailable"))?;
        let mut images = HashMap::new();
        for (id, hash) in wanted {
            let (asset, bytes) = project
                .read_asset(&id)
                .map_err(|_| not_ready("Image asset bytes are missing or changed"))?;
            if asset.content_hash != hash {
                return Err(not_ready(
                    "Image asset changed since the scene was computed",
                ));
            }
            let image = Image::new(&asset.media_type, bytes)
                .ok_or_else(|| not_ready("Image asset header gives no readable size"))?;
            images.insert(id, image);
        }
        Ok(images)
    })
    .await
    .map_err(|_| ApiError::new("INTERNAL_ERROR", "Storage task failed"))?
}

/// One SVG file. Width and height are the node's size in CSS pixels.
pub fn file(
    scene: &Value,
    images: &HashMap<String, Image>,
    font_css: &str,
    setting: &ExportSetting,
) -> Result<StoredFile> {
    let mut svg = scene_to_svg(scene, images, setting.svg_id_attribute)?;
    // Insert within the SVG root, before painted content. Generated CSS contains no XML delimiters.
    if !font_css.is_empty() {
        let end = svg.markup.find('>').unwrap() + 1;
        svg.markup
            .insert_str(end, &format!("<defs><style>{font_css}</style></defs>"));
    }
    let viewport = &scene["provenance"]["viewport"];
    let bytes = svg.markup.into_bytes();
    let mut meta = json!({"fileName":setting.file_name,"format":"svg","mimeType":"image/svg+xml","width":viewport["width"],"height":viewport["height"],"bytes":bytes.len()});
    let mut diagnostics = Vec::new();
    if svg.has_text {
        diagnostics.push("SVG_TEXT_NOT_OUTLINED");
    }
    // Strokes are not drawn in SVG yet; say so rather than drop them silently.
    let mut nodes = scene["nodes"].as_array().into_iter().flatten();
    if nodes.any(|n| !n["stroke"].is_null()) {
        diagnostics.push("SVG_STROKE_NOT_EXPORTED");
    }
    if scene["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|n| n["shadow"].as_array().is_some_and(|s| !s.is_empty()))
    {
        diagnostics.push("SVG_SHADOW_NOT_EXPORTED");
    }
    if !diagnostics.is_empty() {
        meta["diagnostics"] = json!(diagnostics);
    }
    Ok(StoredFile { meta, bytes })
}
