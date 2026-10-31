//! Embed only validated store-owned font binaries. Never dereference a scene URL.
use super::export_svg::AssetSource;
use crate::error::{ApiError, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use std::fmt::Write;
#[cfg(test)]
#[path = "export_fonts_tests.rs"]
mod tests;

pub async fn embed(source: &AssetSource, scene: &Value) -> Result<(Value, String)> {
    let mut scene = scene.clone();
    let (store, project_id) = (source.store.clone(), source.project_id.clone());
    tokio::task::spawn_blocking(move || {
        let project = store.open_project(&project_id)?;
        let wanted: Vec<String> = scene["nodes"].as_array().into_iter().flatten()
            .filter(|n| n["kind"] == "text")
            .map(|n| n["fontId"].as_str().unwrap_or_default().to_owned()).collect();
        let mut css = String::new();
        let fonts = scene["fonts"].as_array_mut()
            .ok_or_else(|| ApiError::new("RESOURCE_NOT_READY", "Font manifest is unavailable"))?;
        for id in &wanted {
            if !fonts.iter().any(|f| f["id"] == *id) {
                return Err(ApiError::new("RESOURCE_NOT_READY", "Text font is missing from the manifest"));
            }
        }
        for manifest in fonts.iter_mut().filter(|f| wanted.iter().any(|id| f["id"] == *id)) {
            let id = manifest["id"].as_str().unwrap_or_default();
            let (font, mime, bytes) = project.read_font(id)?;
            let hash = font.content_hash.as_deref().unwrap_or_default();
            if manifest["hash"] != hash || manifest["weight"] != font.weight || manifest["style"] != font.style {
                return Err(ApiError::new("RESOURCE_NOT_READY", "Font changed since scene computation"));
            }
            let family = format!("oc_{hash}_{}_{}", font.weight, font.style);
            // Family/hash/style/weight come from validated records; data is base64 only.
            let _ = write!(css, "@font-face{{font-family:'{family}';font-style:{};font-weight:{};src:url(data:{mime};base64,{})}}", font.style, font.weight, STANDARD.encode(bytes));
            manifest["family"] = Value::String(family);
            if css.len() > 40 * 1024 * 1024 {
                return Err(ApiError::new("PIXEL_BUDGET", "Embedded fonts exceed the SVG resource budget"));
            }
        }
        Ok((scene, css))
    }).await.map_err(|_| ApiError::new("INTERNAL_ERROR", "Font storage task failed"))?
}
