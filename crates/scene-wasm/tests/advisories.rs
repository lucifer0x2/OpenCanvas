use opencanvas_scene_wasm::SceneEngine;
use serde_json::{json, Value};

const FONT: &[u8] = include_bytes!("../../../renderer/fonts/NotoSans.ttf");
const HASH: &str = "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d";

fn snapshot() -> Value {
    json!({
        "schemaVersion":4,"projectId":"advisories","slug":"test","name":"Test","revision":7,
        "nodes":[
            {"id":"page","parentId":null,"sortKey":0,"kind":"page","props":{}},
            {"id":"frame","parentId":"page","sortKey":0,"kind":"frame",
                "props":{"width":200,"height":100,"clip":true}},
            {"id":"text","parentId":"frame","sortKey":0,"kind":"text",
                "props":{"text":"PRIVATE_COPY","fontId":"font","fontSize":16,"lineHeight":24,
                    "width":120,"height":1,"x":180,"y":10}}
        ],
        "tokens":[],"styles":[],"assets":[],
        "fonts":[{"id":"font","family":"Noto Sans","weight":400,"style":"normal",
            "assetId":null,"bundledId":"noto","contentHash":HASH,"license":"OFL","status":"ready"}],
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"bounded"}
    })
}

fn compute(snapshot: &Value) -> Value {
    let mut engine = SceneEngine::new();
    engine.load_font("font", HASH, FONT).unwrap();
    serde_json::from_str(
        &engine
            .compute_json(&snapshot.to_string(), "frame", 200., 100., "default")
            .unwrap(),
    )
    .unwrap()
}

fn overflow(scene: &Value) -> Vec<&Value> {
    scene["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"] == "TEXT_CONTENT_OUTSIDE_BOX")
        .collect()
}

#[test]
fn json_advisories_keep_measured_evidence_and_the_emitted_scene_provenance() {
    let scene = compute(&snapshot());
    let items = overflow(&scene);
    assert_eq!(items.len(), 1);
    let item = items[0];
    assert_eq!(item["nodeId"], "text");
    assert_eq!(item["severity"], "warning");
    assert_eq!(item["intentionalOverflow"], false);
    assert_eq!(item["inkEvidence"], "measured");
    assert_eq!(item["examples"][0]["clipNodeIds"], json!(["frame"]));
    assert_eq!(
        item["sourceSceneKey"],
        scene["provenance"]["sourceSceneKey"]
    );
    let summary = scene["diagnostics"].as_array().unwrap().last().unwrap();
    assert_eq!(summary["code"], "TEXT_OVERFLOW_SUMMARY");
    assert_eq!(summary["reportedNodes"], 1);
    assert_eq!(summary["omittedNodes"], 0);
    assert_eq!(summary["provenance"], scene["provenance"]);
    assert_ne!(scene["provenance"]["engine"], "native");
    assert!(!scene["diagnostics"].to_string().contains("PRIVATE_COPY"));
}

#[test]
fn note_intent_on_text_and_ancestors_changes_only_advisory_severity() {
    let base = snapshot();
    let original = compute(&base);
    for id in ["text", "frame"] {
        let mut snapshot = base.clone();
        snapshot["notes"] = json!([{
            "id":"note","nodeId":id,"text":"Intentional Overflow: PRIVATE_NOTE"
        }]);
        let scene = compute(&snapshot);
        let item = overflow(&scene)[0];
        assert_eq!(item["severity"], "info");
        assert_eq!(item["intentionalOverflow"], true);
        assert_eq!(scene["nodes"], original["nodes"]);
        assert!(!scene["diagnostics"].to_string().contains("PRIVATE_NOTE"));
    }
}

#[test]
fn master_and_instance_notes_apply_to_resolved_placed_text() {
    let mut base = snapshot();
    base["nodes"][1]["id"] = json!("master");
    base["nodes"][1]["props"]["component"] = json!({"name":"Overflow"});
    base["nodes"][2]["parentId"] = json!("master");
    base["nodes"].as_array_mut().unwrap().extend([
        json!({"id":"frame","parentId":"page","sortKey":1,"kind":"frame",
            "props":{"width":200,"height":100}}),
        json!({"id":"placed","parentId":"frame","sortKey":0,"kind":"instance",
            "props":{"componentId":"master"}}),
    ]);
    let original = compute(&base);
    assert_eq!(overflow(&original)[0]["nodeId"], "placed/text");
    for id in ["master", "text", "placed"] {
        let mut snapshot = base.clone();
        snapshot["notes"] = json!([{
            "id":"note","nodeId":id,"text":"intentional overflow: placed design"
        }]);
        let scene = compute(&snapshot);
        assert_eq!(overflow(&scene)[0]["intentionalOverflow"], true, "{id}");
        assert_eq!(scene["nodes"], original["nodes"]);
    }
}
