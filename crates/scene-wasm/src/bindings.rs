use crate::{ResourceUrls, SceneEngine, SceneOptions, Snapshot, StorageError};
use wasm_bindgen::prelude::*;

/// A single dedicated Worker owns this object and its verified font cache.
#[wasm_bindgen(js_name = SceneComputer)]
pub struct SceneComputer(SceneEngine);

#[wasm_bindgen(js_class = SceneComputer)]
impl SceneComputer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(SceneEngine::new())
    }

    pub fn load_font(&mut self, id: &str, hash: &str, bytes: &[u8]) -> Result<(), JsValue> {
        self.0.load_font(id, hash, bytes).map_err(js_error)
    }

    pub fn set_resource_urls(&mut self, json: &str) -> Result<(), JsValue> {
        let urls: ResourceUrls = serde_json::from_str(json)
            .map_err(StorageError::from)
            .map_err(js_error)?;
        self.0.set_resource_urls(urls);
        Ok(())
    }

    pub fn required_fonts(
        &self,
        snapshot_json: &str,
        frame_id: &str,
        width: f64,
        height: f64,
        theme: &str,
    ) -> Result<String, JsValue> {
        let snapshot: Snapshot = serde_json::from_str(snapshot_json)
            .map_err(StorageError::from)
            .map_err(js_error)?;
        self.0
            .required_fonts(
                &snapshot,
                &SceneOptions {
                    frame_id: frame_id.into(),
                    width,
                    height,
                    theme: theme.into(),
                    state: "default".into(),
                },
            )
            .map_err(js_error)
    }

    /// `[{frameId, digest} | {frameId, error}]` for `[{frameId, width, height}]`, one pass.
    pub fn frame_keys(
        &self,
        snapshot_json: &str,
        frames_json: &str,
        theme: &str,
    ) -> Result<String, JsValue> {
        crate::frame_keys_json(snapshot_json, frames_json, theme).map_err(js_error)
    }

    pub fn compute_scene(
        &self,
        snapshot_json: &str,
        frame_id: &str,
        width: f64,
        height: f64,
        theme: &str,
    ) -> Result<String, JsValue> {
        self.0
            .compute_json(snapshot_json, frame_id, width, height, theme)
            .map_err(js_error)
    }
}

// Plain objects survive Worker structured clone without losing the pipeline code.
fn js_error(error: StorageError) -> JsValue {
    js_sys::JSON::parse(&serde_json::to_string(&error).expect("error JSON is serializable"))
        .expect("serialized error is valid JSON")
}
