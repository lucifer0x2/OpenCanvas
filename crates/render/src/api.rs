use crate::wire::WireScene;
use opencanvas_layout::{Asset, Font, Provenance, SceneNode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Wire scene deliberately does NOT deserialize `layout::Scene`: runtime diagnostics
/// contain objects, and future runtime provenance fields must survive round trips.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(from = "WireScene", into = "WireScene")]
pub struct PaintScene {
    pub schema_version: u32,
    pub coordinate_space: String,
    pub provenance: Provenance,
    pub nodes: Vec<SceneNode>,
    pub fonts: Vec<Font>,
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub diagnostics: Vec<Value>,
    /// Unknown provenance fields preserved on deserialize/serialize.
    #[serde(skip)]
    pub runtime_provenance: BTreeMap<String, Value>,
}

impl From<opencanvas_layout::Scene> for PaintScene {
    fn from(s: opencanvas_layout::Scene) -> Self {
        Self {
            schema_version: s.schema_version,
            coordinate_space: s.coordinate_space,
            provenance: s.provenance,
            nodes: s.nodes,
            fonts: s.fonts,
            assets: s.assets,
            diagnostics: s.diagnostics.into_iter().map(Value::String).collect(),
            runtime_provenance: BTreeMap::new(),
        }
    }
}

/// Cancellation/deadline is supplied by the caller; true aborts with CANCELLED.
/// The callback must be cheap. Codec calls are bounded but not interruptible inside
/// their decoder; checks surround them and occur within our own pixel/blur loops.
pub struct RenderOptions<'a> {
    pub scale: f64,
    pub strict: bool,
    pub max_layer_pixels: u64,
    pub cancelled: Option<&'a dyn Fn() -> bool>,
    /// Experimental glyph-mask exponent; 1 is unmodified geometric coverage.
    pub text_coverage_gamma: f32,
    /// Experimental CoreText-like quarter-pixel x / whole-pixel baseline snapping.
    pub snap_glyph_origins: bool,
}
impl Default for RenderOptions<'_> {
    fn default() -> Self {
        Self {
            scale: 1.0,
            strict: true,
            max_layer_pixels: 16_000_000,
            cancelled: None,
            text_coverage_gamma: 1.0,
            snap_glyph_origins: false,
        }
    }
}
impl RenderOptions<'_> {
    pub(crate) fn check(&self) -> Result<(), RenderError> {
        if self.cancelled.is_some_and(|f| f()) {
            Err(RenderError::new("CANCELLED", "Render cancelled by caller"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawStats {
    pub drawn_lines: usize,
    pub opacity_layers: usize,
    pub opacity_layer_fallbacks: usize,
}
#[derive(Clone, Debug)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    /// Row-major sRGB RGBA8, PREMULTIPLIED alpha, stride = width * 4.
    pub rgba: Vec<u8>,
    pub draw: DrawStats,
    pub source_scene_key: String,
    /// Readiness and advisory diagnostics (MISSING_GLYPH never fails strict).
    pub diagnostics: Vec<Value>,
    pub readiness: Vec<ResourceReadiness>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceReadiness {
    pub id: String,
    pub kind: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderError {
    pub code: String,
    pub message: String,
    /// Decode observations retained when strict painting cannot return a raster.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readiness: Vec<ResourceReadiness>,
}
impl RenderError {
    pub(crate) fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            readiness: Vec::new(),
        }
    }
    pub(crate) fn budget() -> Self {
        Self::new("PIXEL_BUDGET", "Render pixel budget exceeded")
    }
    pub(crate) fn scene(message: &str) -> Self {
        Self::new("SCENE_INVALID", message)
    }
}
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for RenderError {}
