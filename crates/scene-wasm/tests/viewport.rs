use opencanvas_scene_wasm::{SceneEngine, SceneOptions, Snapshot};
use serde_json::json;

fn snapshot() -> Snapshot {
    serde_json::from_value(json!({
        "schemaVersion":4,"projectId":"project","slug":"test","name":"Test","revision":7,
        "nodes":[{"id":"page","parentId":null,"sortKey":0,"kind":"page","props":{}},
            {"id":"frame","parentId":"page","sortKey":0,"kind":"frame",
            "props":{"width":100,"height":100}}],
        "tokens":[],"styles":[],"assets":[],"fonts":[],
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"bounded"}
    }))
    .unwrap()
}

#[test]
fn discovery_and_compute_reject_invalid_viewports_before_resolution() {
    let engine = SceneEngine::new();
    let snapshot = snapshot();
    let snapshot_json = serde_json::to_string(&snapshot).unwrap();
    for (width, height) in [
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
        (f64::NEG_INFINITY, 1.0),
        (0.0, 1.0),
        (1.0, -1.0),
        (100_001.0, 1.0),
        (1.0, 100_001.0),
        (4000.0, 4000.001),
        (8000.0, 8000.0),
    ] {
        // An absent theme would fail resolution if the viewport guard ran late.
        let options = SceneOptions {
            frame_id: "frame".into(),
            width,
            height,
            theme: "absent".into(),
            state: "default".into(),
        };
        let errors = [
            engine.required_fonts(&snapshot, &options).unwrap_err(),
            engine.compute(&snapshot, &options).unwrap_err(),
            engine
                .compute_json(&snapshot_json, "frame", width, height, "absent")
                .unwrap_err(),
        ];
        for error in errors {
            assert_eq!(error.code, "INVALID_REQUEST");
            assert_eq!(error.message, "Viewport or scale exceeds the render budget");
            assert!(!error.retryable);
        }
    }
}

#[test]
fn discovery_and_compute_accept_exact_scale_one_boundaries() {
    let engine = SceneEngine::new();
    let snapshot = snapshot();
    for (width, height) in [(4000.0, 4000.0), (100_000.0, 160.0)] {
        let options = SceneOptions {
            frame_id: "frame".into(),
            width,
            height,
            theme: "default".into(),
            state: "default".into(),
        };
        assert_eq!(engine.required_fonts(&snapshot, &options).unwrap(), "[]");
        let scene = engine.compute(&snapshot, &options).unwrap();
        assert_eq!(
            serde_json::to_value(scene).unwrap()["provenance"]["viewport"],
            json!({
                "width":width,"height":height
            })
        );
    }
}

#[test]
fn a_hug_root_reports_its_content_height_at_any_requested_height() {
    let engine = SceneEngine::new();
    let mut snapshot = serde_json::to_value(snapshot()).unwrap();
    snapshot["nodes"][1]["props"] =
        json!({"width":100,"height":"hug","layout":{"mode":"column","padding":10}});
    snapshot["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"box","parentId":"frame",
        "sortKey":0,"kind":"rectangle","props":{"width":20,"height":30}}));
    let json = snapshot.to_string();
    for height in [1.0, 844.0] {
        let scene: serde_json::Value = serde_json::from_str(
            &engine
                .compute_json(&json, "frame", 100.0, height, "default")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            scene["provenance"]["viewport"],
            json!({"width":100.0,"height":50.0})
        );
        assert_eq!(scene["nodes"][0]["bounds"]["height"], 50.0);
    }
}
