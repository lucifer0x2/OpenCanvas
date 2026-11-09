use opencanvas_layout::{LayoutInput, Scene};
use opencanvas_scene_input::{
    make_input, scene_diagnostics, Node, SceneOptions, Snapshot, StorageError,
};
use opencanvas_text::{Font, FontSet, FontSetMeasurer, TextError};
use serde::Deserialize;
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, StorageError>;

/// Opaque transport labels supplied by the caller; never constructed or fetched here.
/// Replacing the maps prevents stale URLs leaking across snapshot/project changes.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceUrls {
    #[serde(default)]
    pub fonts: BTreeMap<String, String>,
    #[serde(default)]
    pub assets: BTreeMap<String, String>,
}

/// Caller-owned font cache; immutable verified font data is reused between computes.
#[derive(Default)]
pub struct SceneEngine {
    fonts: BTreeMap<String, Font>,
    urls: ResourceUrls,
}

impl SceneEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate original upload hash and decode before replacing any existing font.
    /// Face descriptors are bound to the exact snapshot manifest during compute.
    pub fn load_font(&mut self, id: &str, hash: &str, bytes: &[u8]) -> Result<()> {
        if id.is_empty() || id.len() > 1024 || id.chars().any(char::is_control) {
            return Err(StorageError::validation("fontId", "Invalid font identity"));
        }
        let font = Font::from_bytes(bytes, hash).map_err(text_error)?;
        self.fonts.insert(id.to_owned(), font);
        Ok(())
    }

    pub fn set_resource_urls(&mut self, urls: ResourceUrls) {
        self.urls = urls;
    }

    /// Resolve required faces before fetching bytes, so unused resources never block.
    pub fn required_fonts(&self, snapshot: &Snapshot, options: &SceneOptions) -> Result<String> {
        let input: LayoutInput = serde_json::from_value(make_input(snapshot, options)?)?;
        Ok(serde_json::to_string(&input.fonts)?)
    }

    pub fn compute(&self, snapshot: &Snapshot, options: &SceneOptions) -> Result<Scene> {
        let value = make_input(snapshot, options)?;
        self.compute_input(value)
    }

    fn compute_input(&self, value: serde_json::Value) -> Result<Scene> {
        let mut input: LayoutInput = serde_json::from_value(value)?;
        for font in &mut input.fonts {
            font.url = self.urls.fonts.get(&font.id).cloned().unwrap_or_default();
        }
        for asset in &mut input.assets {
            asset.url = self.urls.assets.get(&asset.id).cloned().unwrap_or_default();
        }
        let mut fonts = FontSet::new();
        for manifest in &input.fonts {
            let font = self.fonts.get(&manifest.id).ok_or_else(|| {
                StorageError::new("FONT_MISSING", "Exact project font bytes are unavailable")
            })?;
            if font.hash() != manifest.hash {
                return Err(StorageError::new(
                    "RESOURCE_HASH_MISMATCH",
                    "Loaded font no longer matches the requested manifest",
                ));
            }
            let font = font
                .clone()
                .with_face(&manifest.family, manifest.weight, &manifest.style)
                .map_err(text_error)?;
            fonts.insert(&manifest.id, font).map_err(text_error)?;
        }
        opencanvas_layout::compute_with_measurer(&input, &mut FontSetMeasurer::new(&fonts))
            .map_err(|error| StorageError::new(&error.code, &error.message))
    }

    pub fn compute_json(
        &self,
        snapshot_json: &str,
        frame_id: &str,
        width: f64,
        height: f64,
        theme: &str,
    ) -> Result<String> {
        let snapshot: Snapshot = serde_json::from_str(snapshot_json)?;
        let options = SceneOptions {
            frame_id: frame_id.into(),
            width,
            height,
            theme: theme.into(),
            state: "default".into(),
        };
        let input = make_input(&snapshot, &options)?;
        let intentional = if snapshot.notes.is_empty() {
            Vec::new()
        } else {
            let nodes: Vec<Node> = Deserialize::deserialize(&input["nodes"])?;
            scene_diagnostics::overflow_ids(&snapshot, &nodes)
        };
        let mut scene = serde_json::to_value(self.compute_input(input)?)?;
        scene_diagnostics::append(&mut scene, &intentional);
        Ok(serde_json::to_string(&scene)?)
    }
}

fn text_error(error: TextError) -> StorageError {
    StorageError::new(error.code(), "Text resources or measurement are invalid")
}
