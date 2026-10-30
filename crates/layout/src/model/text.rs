//! Browser measurement wire contract. Defaults permit old cache fixtures to deserialize.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letter_spacing: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_transform: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextLine {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    #[serde(default, rename = "inkBounds", skip_serializing_if = "Option::is_none")]
    pub ink_bounds: Option<super::Bounds>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measurement {
    pub width: f32,
    pub height: f32,
    pub lines: Vec<TextLine>,
    #[serde(default)]
    pub ascent: f32,
    #[serde(default)]
    pub missing: Vec<u32>,
    #[serde(default)]
    pub coverage_unavailable: bool,
}
pub type MeasureCache = BTreeMap<String, Measurement>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneDecoration {
    pub line: String,
    pub offset: f32,
    pub thickness: f32,
}
