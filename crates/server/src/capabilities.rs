//! Machine-readable authored keys, generated from the live validator catalog.
//! Consumer facts describe implemented behavior, never runtime font/asset readiness.
use opencanvas_storage::{allowed_props, NODE_KINDS, PROPS};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub(super) const URI: &str = "opencanvas://capabilities";
pub(super) const NAME: &str = "OpenCanvas consumer capabilities";

pub(super) fn text() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| serde_json::to_string(&document()).unwrap())
}

fn document() -> Value {
    let properties: serde_json::Map<String, Value> = PROPS
        .iter()
        .map(|spec| {
            let kinds: Vec<_> = NODE_KINDS
                .iter()
                .filter(|kind| allowed_props(kind).contains(&spec.key))
                .map(|kind| kind.as_str())
                .collect();
            (
                spec.key.into(),
                json!({"authored":true,"kinds":kinds,"consumers":facts(spec.key)}),
            )
        })
        .collect();
    let keys: serde_json::Map<String, Value> = NODE_KINDS
        .iter()
        .map(|kind| (kind.as_str().into(), json!(allowed_props(kind))))
        .collect();
    let mut value = json!({
        "capabilityVersion":1,
        "sceneVersion":pipeline_version("SCENE_VERSION"),
        "rendererVersion":pipeline_version("RENDERER_VERSION"),
        "keysByKind":keys,"properties":properties,
        "statusMeaning":{"supported":"Implemented in this consumer, subject to documented value/kind restrictions","metadata":"Preserved for discovery/export or implementation; does not itself paint pixels","resting":"Only the resting/default result; interaction is not played","diagnostic":"Accepted, omitted or approximate in this consumer with an explicit diagnostic","unclassified":"No consumer claim; integration must classify this catalog addition"},
        "runtime":"Static support only. Read job provenance/readiness/diagnostics for loaded resources and glyph coverage. No implicit font fallback or pixel-parity guarantee.",
        "restrictions":{"stroke":"Solid per-side CSS shorthand; SVG omits stroke with SVG_STROKE_NOT_EXPORTED","shadow":"Box layers only, at most 8; no state shadow. SVG omits shadow with SVG_SHADOW_NOT_EXPORTED.","fill":"Solids on text; boxes also accept linear/radial gradients with 2..16 ordered stops. Gradient stops accept solid colors/color tokens. Fill animation requires a solid resting fill.","radius":"Scalar or CSS 1..4 corner shorthand; overlapping radii scale together.","text":"Plain semantic strings; no mixed-style runs, authored RTL, ellipsis or maxLines. Tracking is signed px; decoration is continuous without skip-ink. SVG keeps measured lines and embedded fonts, not outlines; SVG_TEXT_NOT_OUTLINED.","states":"Flat declared fill/opacity/radius/stroke variants only; render_frame previews one selected state across all declaring nodes, including SVG export; preview supports declared interaction.","animations":"Canvas/PNG/SVG show resting animation props; HTML preview plays declared animation.","layout":"row/column/free with documented finite sizing; no percent dimensions or breakpoints"}
    });
    value["catalogFingerprint"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&value).unwrap())
    ));
    value
}

fn pipeline_version(name: &str) -> &'static str {
    // Compile-time source inclusion keeps this resource in sync without a second list of versions.
    include_str!("../../layout/src/lib.rs")
        .lines()
        .find(|line| line.starts_with(&format!("pub const {name}:")))
        .and_then(|line| line.split('"').nth(1))
        .expect("layout pipeline version")
}

fn facts(key: &str) -> Value {
    let (canvas, preview, svg) = match key {
        "name" | "component" | "exportSettings" | "flowStart" => {
            ("metadata", "metadata", "metadata")
        }
        "links" => ("metadata", "supported", "metadata"),
        "animations" => ("resting", "supported", "resting"),
        "states" => ("supported", "supported", "supported"),
        "stroke" | "shadow" => ("supported", "supported", "diagnostic"),
        "text" => ("supported", "supported", "diagnostic"),
        "visible" | "styleId" | "x" | "y" | "width" | "height" | "minWidth" | "maxWidth"
        | "minHeight" | "maxHeight" | "layout" | "fontId" | "fontSize" | "lineHeight" | "fill"
        | "opacity" | "clip" | "radius" | "assetId" | "fit" | "crop" | "componentId"
        | "overrides" | "letterSpacing" | "textAlign" | "textDecoration" | "textTransform"
        | "fontFamily" | "textWrap" | "fontWeight" | "fontStyle" => {
            ("supported", "supported", "supported")
        }
        _ => ("unclassified", "unclassified", "unclassified"),
    };
    json!({"canvasPng":canvas,"preview":preview,"svg":svg})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_accepted_key_has_consumer_facts_and_actual_kind_keys() {
        let doc: Value = serde_json::from_str(text()).unwrap();
        assert_eq!(doc["properties"].as_object().unwrap().len(), PROPS.len());
        for spec in PROPS {
            assert!(doc["properties"].get(spec.key).is_some());
            assert!(
                !facts(spec.key).to_string().contains("unclassified"),
                "Classify {} against each consumer",
                spec.key
            );
        }
        for kind in NODE_KINDS {
            assert_eq!(
                doc["keysByKind"][kind.as_str()],
                json!(allowed_props(&kind))
            );
        }
        let types = include_str!("../../../renderer/types.ts");
        for key in ["SCENE_VERSION", "RENDERER_VERSION"] {
            assert!(types.contains(&format!("{key} = \"{}\"", pipeline_version(key))));
        }
        assert_eq!(facts("states")["svg"], "supported");
        let svg_export = include_str!("jobs/export_svg.rs");
        for (prop, diagnostic) in [
            ("stroke", "SVG_STROKE_NOT_EXPORTED"),
            ("shadow", "SVG_SHADOW_NOT_EXPORTED"),
            ("text", "SVG_TEXT_NOT_OUTLINED"),
        ] {
            assert_eq!(facts(prop)["svg"], "diagnostic");
            assert!(svg_export.contains(diagnostic));
            assert!(doc["restrictions"].to_string().contains(diagnostic));
        }
    }
}
