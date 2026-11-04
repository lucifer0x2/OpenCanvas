//! Storage-only preparation. URLs in the resolved layout input are never followed.
use super::Control;
use crate::error::{ApiError, Result};
use crate::jobs::resource_bytes::ResourceBytes;
use opencanvas_layout::LayoutInput;
use opencanvas_storage::Storage;
use opencanvas_text::{Font, FontSet, TextError};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Exact measurement faces plus original paint bytes, retained for this whole job.
/// Asset availability/hash observations precede native decode observations; loading
/// must not destroy a usable scene merely because a strict paint resource is missing.
pub(super) struct Resources {
    pub fonts: FontSet,
    pub readiness: Vec<Value>,
    pub diagnostics: Vec<String>,
    pub bytes: ResourceBytes,
}

pub(super) fn load(store: &Storage, input: &LayoutInput, control: &Control) -> Result<Resources> {
    control.check()?;
    let project = store.open_project(&input.project_id)?;
    let mut resources = Resources {
        fonts: FontSet::new(),
        readiness: Vec::new(),
        diagnostics: Vec::new(),
        bytes: ResourceBytes::default(),
    };
    for manifest in &input.fonts {
        control.check()?;
        let (stored, mime, bytes) = project.read_font(&manifest.id).map_err(|_| {
            ApiError::new("FONT_MISSING", "Exact project font bytes are unavailable")
        })?;
        control.check()?;
        if stored.content_hash.as_deref() != Some(&manifest.hash) {
            return Err(ApiError::new(
                "RESOURCE_HASH_MISMATCH",
                "Project font no longer matches the requested manifest",
            ));
        }
        if stored.family != manifest.family
            || stored.weight != manifest.weight
            || stored.style != manifest.style
        {
            return Err(ApiError::new(
                "FONT_MISSING",
                "Exact project font face is unavailable",
            ));
        }
        let font = Font::from_bytes(&bytes, &manifest.hash)
            .and_then(|font| font.with_face(&manifest.family, manifest.weight, &manifest.style))
            .map_err(text_error)?;
        control.check()?;
        resources
            .fonts
            .insert(&manifest.id, font)
            .map_err(text_error)?;
        resources
            .bytes
            .fonts
            .insert(manifest.id.clone(), (stored, mime, bytes));
        resources
            .readiness
            .push(json!({"id":manifest.id,"kind":"font","status":"ready"}));
    }
    // Missing paint data cannot destroy a usable computed scene. Do not claim image
    // decode success here: record failures only, then let native preparation decide.
    for manifest in &input.assets {
        control.check()?;
        let failure = match project.read_asset(&manifest.id) {
            Ok((asset, bytes)) if format!("{:x}", Sha256::digest(&bytes)) == manifest.hash => {
                resources
                    .bytes
                    .assets
                    .insert(manifest.id.clone(), (asset, bytes));
                None
            }
            Ok(_) => Some("RESOURCE_HASH_MISMATCH"),
            Err(_) => Some("RESOURCE_UNAVAILABLE"),
        };
        control.check()?;
        if let Some(code) = failure {
            resources
                .readiness
                .push(json!({"id":manifest.id,"kind":"asset","status":"failed","code":code}));
            resources
                .diagnostics
                .push(format!("Asset {} failed readiness", manifest.id));
        }
    }
    resources.readiness.sort_by_key(|value| {
        (
            value["kind"].as_str().unwrap().to_owned(),
            value["id"].as_str().unwrap().to_owned(),
        )
    });
    Ok(resources)
}

pub(super) fn text_error(error: TextError) -> ApiError {
    ApiError::new(
        error.code(),
        "Native text resources or measurement are invalid",
    )
}
