use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod layout;
mod stroke;
pub use layout::{LayoutRule, Padding, Sizing};
pub use stroke::{SceneStroke, Sides, StrokeInput};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Font {
    pub id: String,
    pub family: String,
    pub hash: String,
    pub url: String,
    #[serde(default = "weight")]
    pub weight: u16,
    #[serde(default = "normal")]
    pub style: String,
}
fn weight() -> u16 {
    400
}
fn normal() -> String {
    "normal".into()
}
fn default_theme() -> String {
    "default".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub hash: String,
    pub url: String,
    pub media_type: String,
}

/// Normalized source-image rectangle, before contain/cover/fill.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Crop {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Props {
    pub name: Option<String>,
    pub width: Option<Sizing>,
    pub height: Option<Sizing>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub layout: Option<LayoutRule>,
    pub text: Option<String>,
    pub font_id: Option<String>,
    pub font_size: Option<f32>,
    pub line_height: Option<f32>,
    pub fill: Option<String>,
    pub asset_id: Option<String>,
    pub fit: Option<String>,
    pub opacity: Option<f32>,
    pub clip: Option<bool>,
    pub radius: Option<f32>,
    pub crop: Option<Crop>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<StrokeInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub props: Props,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutInput {
    pub schema_version: u32,
    pub project_id: String,
    pub revision: u64,
    pub frame_id: String,
    pub viewport: Viewport,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_theme")]
    pub state: String,
    pub nodes: Vec<AuthoredNode>,
    #[serde(default)]
    pub fonts: Vec<Font>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasureRequest {
    pub key: String,
    pub text: String,
    pub font_id: String,
    pub font_hash: String,
    pub family: String,
    pub weight: u16,
    pub style: String,
    pub font_size: f32,
    pub line_height: f32,
    pub width: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextLine {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Measurement {
    pub width: f32,
    pub height: f32,
    pub lines: Vec<TextLine>,
}
pub type MeasureCache = BTreeMap<String, Measurement>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: String,
    pub bounds: Bounds,
    pub lines: Vec<TextLine>,
    pub font_id: Option<String>,
    pub font_size: f32,
    pub line_height: f32,
    pub fill: String,
    pub asset_id: Option<String>,
    pub fit: String,
    pub opacity: f32,
    pub clip: bool,
    pub radius: f32,
    pub crop: Option<Crop>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<SceneStroke>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub project_id: String,
    pub revision: u64,
    pub frame_id: String,
    pub viewport: Viewport,
    pub theme: String,
    pub state: String,
    pub scene_version: String,
    pub renderer_version: String,
    pub source_scene_key: String,
    pub font_manifest_hash: String,
    pub asset_manifest_hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub schema_version: u32,
    pub coordinate_space: String,
    pub provenance: Provenance,
    pub nodes: Vec<SceneNode>,
    pub fonts: Vec<Font>,
    pub assets: Vec<Asset>,
    pub diagnostics: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
#[expect(
    clippy::large_enum_variant,
    reason = "Preserve the existing public LayoutStep::Ready { scene: Scene } API; boxing is an unrelated breaking change"
)]
pub enum LayoutStep {
    Pending { requests: Vec<MeasureRequest> },
    Ready { scene: Scene },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutError {
    pub code: String,
    pub message: String,
}
impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for LayoutError {}
