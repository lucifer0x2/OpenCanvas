use opencanvas_scene_wasm::{ResourceUrls, SceneEngine};
use serde_json::{json, Value};

const FONT: &[u8] = include_bytes!("../../../renderer/fonts/NotoSans.ttf");
const HASH: &str = "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d";

fn snapshot() -> Value {
    json!({
        "schemaVersion":4,"projectId":"project","slug":"test","name":"Test","revision":7,
        "nodes":[
            {"id":"page","parentId":null,"sortKey":0,"kind":"page","props":{}},
            {"id":"frame","parentId":"page","sortKey":0,"kind":"frame",
             "props":{"width":200,"height":120,"layout":{"mode":"column","gap":8}}},
            {"id":"text","parentId":"frame","sortKey":0,"kind":"text",
             "props":{"text":"Tiếng Việt — OpenCanvas wraps exactly","fontId":"font",
                 "fontSize":16,"lineHeight":24,"width":"fill","height":"hug"}}
        ],
        "tokens":[],"styles":[],"assets":[],
        "fonts":[{"id":"font","family":"Noto Sans","weight":400,"style":"normal",
            "assetId":null,"bundledId":"noto","contentHash":HASH,"license":"OFL","status":"ready"}],
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"bounded"}
    })
}

fn compute(engine: &SceneEngine, snapshot: &Value, width: f64) -> Result<Value, String> {
    engine
        .compute_json(&snapshot.to_string(), "frame", width, 120.0, "default")
        .map(|scene| serde_json::from_str(&scene).unwrap())
        .map_err(|error| error.code)
}

#[test]
fn verified_fonts_produce_wrapped_lines_and_transport_consistent_hashes() {
    let mut engine = SceneEngine::new();
    engine.load_font("font", HASH, FONT).unwrap();
    let original = compute(&engine, &snapshot(), 200.0).unwrap();
    let narrow = compute(&engine, &snapshot(), 80.0).unwrap();
    assert!(
        narrow["nodes"][1]["lines"].as_array().unwrap().len()
            > original["nodes"][1]["lines"].as_array().unwrap().len()
    );
    assert_eq!(original["provenance"]["sceneVersion"], "scene-3");
    assert_eq!(original["provenance"]["rendererVersion"], "web-4");
    assert_eq!(original["provenance"]["state"], "default");
    assert_eq!(original["fonts"][0]["url"], "");

    let urls: ResourceUrls = serde_json::from_value(json!({
        "fonts":{"font":"/caller/font.woff2"},"assets":{}
    }))
    .unwrap();
    engine.set_resource_urls(urls);
    let decorated = compute(&engine, &snapshot(), 200.0).unwrap();
    assert_eq!(decorated["fonts"][0]["url"], "/caller/font.woff2");
    assert_eq!(original["nodes"], decorated["nodes"]);
    assert_ne!(
        original["provenance"]["fontManifestHash"],
        decorated["provenance"]["fontManifestHash"]
    );
    assert_ne!(
        original["provenance"]["sourceSceneKey"],
        decorated["provenance"]["sourceSceneKey"]
    );
}

#[test]
fn missing_mismatched_and_rejected_font_loads_keep_exact_identity() {
    let mut engine = SceneEngine::new();
    assert_eq!(
        compute(&engine, &snapshot(), 200.0).unwrap_err(),
        "FONT_MISSING"
    );
    engine.load_font("font", HASH, FONT).unwrap();
    let invalid = engine.load_font("font", HASH, b"corrupt").unwrap_err();
    assert_eq!(invalid.code, "RESOURCE_HASH_MISMATCH");
    assert!(compute(&engine, &snapshot(), 200.0).is_ok());
    let mut mismatch = snapshot();
    mismatch["fonts"][0]["contentHash"] = json!("0".repeat(64));
    assert_eq!(
        compute(&engine, &mismatch, 200.0).unwrap_err(),
        "RESOURCE_HASH_MISMATCH"
    );
}

#[test]
fn structured_validation_errors_do_not_publish_partial_scenes() {
    let engine = SceneEngine::new();
    let invalid = engine
        .compute_json("{", "frame", 100.0, 100.0, "default")
        .unwrap_err();
    assert_eq!(invalid.code, "VALIDATION_FAILED");
    let invalid = engine
        .compute_json(&snapshot().to_string(), "absent", 100.0, 100.0, "default")
        .unwrap_err();
    assert_eq!(invalid.code, "INVALID_REQUEST");
    assert!(compute(&engine, &snapshot(), f64::NAN).is_err());
}

#[test]
fn unused_fonts_do_not_block_nontext_scenes() {
    let mut snapshot = snapshot();
    snapshot["nodes"].as_array_mut().unwrap().pop();
    assert!(compute(&SceneEngine::new(), &snapshot, 200.0).is_ok());
}
