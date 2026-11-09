use super::{check_request, make_input};
use crate::jobs::SceneRequest;
use opencanvas_storage::Snapshot;
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

fn request(width: f64, height: f64, scale: f64) -> SceneRequest {
    SceneRequest {
        frame_id: "frame".into(),
        revision: 7,
        width,
        height,
        scale,
        theme: "default".into(),
        state: "default".into(),
    }
}

#[test]
fn server_preserves_scaled_budget_boundaries_through_input_assembly() {
    let snapshot = snapshot();
    for (width, height, scale) in [
        (4000.0, 4000.0, 1.0),
        (2000.0, 2000.0, 2.0),
        (1000.0, 1000.0, 4.0),
        (8000.0, 8000.0, 0.5),
        (100_000.0, 160.0, 1.0),
    ] {
        let request = request(width, height, scale);
        check_request(&snapshot, &request).unwrap();
        let input = make_input(&snapshot, &request, "").unwrap();
        assert_eq!(input["viewport"], json!({"width":width,"height":height}));
        let mut over = request;
        over.height += 0.001;
        for error in [
            check_request(&snapshot, &over).unwrap_err(),
            make_input(&snapshot, &over, "").unwrap_err(),
        ] {
            assert_eq!(error.0.code, "INVALID_REQUEST");
            assert_eq!(
                error.0.message,
                "Viewport or scale exceeds the render budget"
            );
            assert!(!error.0.retryable);
        }
    }
}

#[test]
fn server_keeps_revision_and_state_validation_order() {
    let snapshot = snapshot();
    let mut request = request(f64::NAN, 1.0, 1.0);
    request.revision = 6;
    assert_eq!(
        check_request(&snapshot, &request).unwrap_err().0.code,
        "REVISION_UNAVAILABLE"
    );
    request.revision = 7;
    request.state = "unknown".into();
    assert_eq!(
        check_request(&snapshot, &request).unwrap_err().0.message,
        "Viewport or scale exceeds the render budget"
    );
    request.width = 1.0;
    assert_eq!(
        check_request(&snapshot, &request).unwrap_err().0.message,
        "Unknown interaction state"
    );
}
