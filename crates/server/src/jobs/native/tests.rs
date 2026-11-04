use super::*;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[test]
fn storage_font_compute_is_one_pass_and_never_follows_scene_urls() {
    let root = tempfile::tempdir().unwrap();
    let store: Storage = opencanvas_storage::Store::open(root.path()).unwrap().into();
    let project = store.create_project("native", "Native").unwrap();
    let snapshot = project.snapshot().unwrap();
    let font = &snapshot.fonts[0];
    let input = json!({"schemaVersion":1,"projectId":snapshot.project_id,"revision":0,
        "frameId":"frame","viewport":{"width":320,"height":200},
        "nodes":[{"id":"frame","kind":"frame","parentId":null,"props":{"width":320,"height":200}},
            {"id":"text","kind":"text","parentId":"frame","props":{"text":"Tiếng Việt 🦄","width":200,"height":"hug","fontId":font.id,"fontSize":20,"lineHeight":28}}],
        "fonts":[{"id":font.id,"family":font.family,"hash":font.content_hash,
            "weight":font.weight,"style":font.style,"url":"file:///never-read-font"}],
        "assets":[{"id":"missing","hash":"0".repeat(64),"url":"file:///never-read-image","mediaType":"image/png"}]});
    let computed = compute_sync(
        &store,
        input.clone(),
        Control::new(Instant::now() + Duration::from_secs(5)),
    )
    .unwrap()
    .value;
    assert_eq!(computed["metrics"]["layoutPasses"], 1);
    assert_eq!(computed["metrics"]["batchRPCs"], 0);
    assert!(computed["metrics"]["measuredItems"].as_u64().unwrap() > 0);
    assert!(computed["metrics"]["elapsedMs"].as_f64().unwrap() >= 0.);
    assert_eq!(computed["readiness"][0]["status"], "failed");
    assert_eq!(computed["readiness"][1]["status"], "ready");
    let mut wrong_hash = input.clone();
    wrong_hash["fonts"][0]["hash"] = json!("0".repeat(64));
    let error = compute_sync(
        &store,
        wrong_hash,
        Control::new(Instant::now() + Duration::from_secs(5)),
    )
    .unwrap_err();
    assert_eq!(error.0.code, "RESOURCE_HASH_MISMATCH");
    assert!(!error
        .value()
        .to_string()
        .contains(root.path().to_str().unwrap()));
    let mut wrong_face = input;
    wrong_face["fonts"][0]["weight"] = json!(700);
    let error = compute_sync(
        &store,
        wrong_face,
        Control::new(Instant::now() + Duration::from_secs(5)),
    )
    .unwrap_err();
    assert_eq!(error.0.code, "FONT_MISSING");
}

#[tokio::test]
async fn deadline_drains_owned_work_and_aborted_future_keeps_lane_guard() {
    let finished = Arc::new(AtomicBool::new(false));
    let end = finished.clone();
    let error = blocking(Instant::now() + Duration::from_millis(30), move |_| {
        std::thread::sleep(Duration::from_millis(70));
        end.store(true, Ordering::SeqCst);
        Ok(())
    })
    .await
    .unwrap_err();
    assert_eq!(error.0.code, "JOB_TIMEOUT");
    assert!(
        finished.load(Ordering::SeqCst),
        "timeout must drain before returning"
    );

    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (cancelled_tx, cancelled_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(blocking(
        Instant::now() + Duration::from_secs(2),
        move |control| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            cancelled_tx
                .send(control.check().unwrap_err().0.code.clone())
                .unwrap();
            Ok(())
        },
    ));
    started_rx.await.unwrap();
    task.abort();
    let _ = task.await;
    let entered = Arc::new(AtomicBool::new(false));
    let marker = entered.clone();
    let error = blocking(Instant::now() + Duration::from_millis(20), move |_| {
        marker.store(true, Ordering::SeqCst);
        Ok(())
    })
    .await
    .unwrap_err();
    assert_eq!(error.0.code, "JOB_TIMEOUT");
    assert!(
        !entered.load(Ordering::SeqCst),
        "cancelled work still owns the lane"
    );
    release_tx.send(()).unwrap();
    assert_eq!(cancelled_rx.await.unwrap(), "JOB_CANCELLED");
    blocking(Instant::now() + Duration::from_secs(2), |_| Ok(()))
        .await
        .unwrap();
}
