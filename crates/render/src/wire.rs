//! Lossless unknown runtime provenance without changing layout's public Scene types.
use crate::api::PaintScene;
use opencanvas_layout::{Asset, Font, Provenance, SceneNode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Deserialize)]
struct WireProvenance {
    #[serde(flatten)]
    known: Provenance,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireScene {
    schema_version: u32,
    coordinate_space: String,
    provenance: WireProvenance,
    nodes: Vec<SceneNode>,
    fonts: Vec<Font>,
    assets: Vec<Asset>,
    #[serde(default)]
    diagnostics: Vec<Value>,
}
impl From<WireScene> for PaintScene {
    fn from(s: WireScene) -> Self {
        Self {
            schema_version: s.schema_version,
            coordinate_space: s.coordinate_space,
            provenance: s.provenance.known,
            runtime_provenance: s.provenance.extra,
            nodes: s.nodes,
            fonts: s.fonts,
            assets: s.assets,
            diagnostics: s.diagnostics,
        }
    }
}
impl From<PaintScene> for WireScene {
    fn from(s: PaintScene) -> Self {
        Self {
            schema_version: s.schema_version,
            coordinate_space: s.coordinate_space,
            provenance: WireProvenance {
                known: s.provenance,
                extra: s.runtime_provenance,
            },
            nodes: s.nodes,
            fonts: s.fonts,
            assets: s.assets,
            diagnostics: s.diagnostics,
        }
    }
}
