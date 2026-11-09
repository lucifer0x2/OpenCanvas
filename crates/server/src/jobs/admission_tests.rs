use super::*;
use crate::{jobs::SceneRequest, state::Config};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{mpsc, watch, Mutex};

fn fixture() -> (Jobs, Snapshot) {
    let (doorbell, _) = mpsc::channel(1);
    let jobs = Jobs {
        table: Mutex::new(Table::default()),
        doorbell,
        config: Arc::new(Config::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            0,
        )),
        done: watch::channel(0).0,
    };
    let mut nodes = vec![json!({"id":"page","parentId":null,"kind":"page","sortKey":0,"props":{}})];
    for index in 0..4 {
        nodes.push(json!({"id":format!("f{index}"),"parentId":"page","kind":"frame","sortKey":index,"props":{"width":100,"height":100}}));
    }
    let snapshot = serde_json::from_value(
        json!({"schemaVersion":1,"projectId":"p","slug":"p","name":"P","revision":1,
        "nodes":nodes,"tokens":[],"styles":[],"assets":[],"fonts":[],
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"retain"}}),
    )
    .unwrap();
    (jobs, snapshot)
}

fn requests(width: f64) -> Vec<SceneRequest> {
    (0..4)
        .map(|index| SceneRequest {
            frame_id: format!("f{index}"),
            revision: 1,
            width,
            height: 100.0,
            theme: "default".into(),
            state: "default".into(),
            scale: 1.0,
        })
        .collect()
}

#[tokio::test]
async fn a_bad_later_frame_admits_nothing() {
    let (jobs, snapshot) = fixture();
    let mut requests = requests(200.0);
    requests[3].width = 100001.0;
    assert_eq!(
        jobs.enqueue_batch(&snapshot, requests)
            .await
            .unwrap_err()
            .value()["error"]["code"],
        "INVALID_REQUEST"
    );
    let table = jobs.table.lock().await;
    assert!(table.entries.is_empty());
    assert_eq!(table.queue.direct_len(), 0);
}

#[tokio::test]
async fn insufficient_batch_capacity_admits_no_partial_jobs_and_retry_reuses_ids() {
    let (jobs, snapshot) = fixture();
    let first = jobs
        .enqueue_batch(&snapshot, requests(200.0))
        .await
        .unwrap();
    let mut partial = requests(201.0);
    partial.truncate(2);
    jobs.enqueue_batch(&snapshot, partial).await.unwrap();
    let error = jobs
        .enqueue_batch(&snapshot, requests(300.0))
        .await
        .unwrap_err()
        .value();
    assert_eq!(error["error"]["code"], "QUEUE_FULL");
    assert_eq!(error["error"]["retryable"], true);
    assert!(error["error"]["retryAfterMs"].as_u64().unwrap() > 0);
    {
        let table = jobs.table.lock().await;
        assert_eq!(table.entries.len(), 6);
        assert_eq!(table.queue.direct_len(), 6);
    }
    let retry = jobs
        .enqueue_batch(&snapshot, requests(200.0))
        .await
        .unwrap();
    assert_eq!(retry, first);
    assert_eq!(jobs.table.lock().await.entries.len(), 6);
}

/// The viewer accepts a job only for its requested revision. Job keys hash the layout input,
/// which carries the revision, so an unchanged frame at a new revision gets a fresh job of that
/// revision instead of a reused job labelled with the old one.
#[tokio::test]
async fn unchanged_frames_get_jobs_of_the_new_revision() {
    let (jobs, snapshot) = fixture();
    let first = jobs
        .enqueue_batch(&snapshot, requests(200.0))
        .await
        .unwrap();
    let mut next = snapshot.clone();
    next.revision = 2;
    let mut later = requests(200.0);
    for request in &mut later {
        request.revision = 2;
    }
    let second = jobs.enqueue_batch(&next, later).await.unwrap();
    for (old, new) in first.iter().zip(&second) {
        assert_eq!(old["revision"], 1);
        assert_eq!(new["revision"], 2);
        assert_ne!(old["jobId"], new["jobId"]);
        assert_ne!(old["sceneKey"], new["sceneKey"]);
    }
}
