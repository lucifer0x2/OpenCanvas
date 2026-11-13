//! Inspection depends only on its immutable retained job, never current storage.
use super::*;
use crate::{jobs::SceneRequest, state::Config};
use opencanvas_storage::{Snapshot, Store};

#[tokio::test]
async fn inspecting_retained_job_does_not_load_a_project_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path()).unwrap();
    let mut config = Config::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        0,
    );
    config.node = "missing-inspect-test-worker".into();
    let state = AppState::new(store, config);
    let snapshot: Snapshot = serde_json::from_value(json!({
        "schemaVersion":1,"projectId":"retained-project","slug":"retained","name":"Retained","revision":9,
        "nodes":[{"id":"page","parentId":null,"kind":"page","sortKey":0,"props":{}},
            {"id":"frame","parentId":"page","kind":"frame","sortKey":0,"props":{"width":10,"height":10}}],
        "tokens":[],"styles":[],"fonts":[],"assets":[],"notes":[],
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"bounded"}
    })).unwrap();
    let job = state
        .jobs
        .enqueue(
            snapshot,
            SceneRequest {
                frame_id: "frame".into(),
                revision: 9,
                width: 10.,
                height: 10.,
                theme: "default".into(),
                state: "default".into(),
                scale: 1.,
            },
        )
        .await
        .unwrap();
    assert!(state.snapshot("retained-project".into()).await.is_err());
    let inspected = inspect(
        &state,
        json!({"projectId":"retained-project","jobId":job["jobId"],"sceneKey":job["sceneKey"]}),
    )
    .await
    .unwrap();
    assert_eq!(inspected["revision"], 9);
    assert_eq!(inspected["sceneKey"], job["sceneKey"]);
}
