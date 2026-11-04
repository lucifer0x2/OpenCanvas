//! Real enqueue → native runner → locked publication → public download guards.
//! The renderer finishes well within budget; only the controlled table lock expires it.
use super::*;
use crate::jobs::{ExportSetting, ExportSpec, SceneRequest, Table};
use opencanvas_storage::{Snapshot, Store};
use std::{future::pending, time::Duration};
use tokio::sync::{watch, Mutex};

fn fixture() -> (tempfile::TempDir, Storage, Snapshot) {
    let root = tempfile::tempdir().unwrap();
    let store: Storage = Store::open(root.path().join("data")).unwrap().into();
    let project = store.create_project("deadline", "Deadline").unwrap();
    let request = serde_json::from_value(json!({"projectId":project.id(),
    "expectedRevision":0,"idempotencyKey":"design","operations":[
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page",
            "props":{"width":10,"height":10,"fill":"#123456"}}
    ]}))
    .unwrap();
    project.apply_batch(request).unwrap();
    let snapshot = project.snapshot().unwrap();
    (root, store, snapshot)
}

fn spec(formats: &[&str]) -> ExportSpec {
    ExportSpec {
        node_id: "frame".into(),
        settings: formats
            .iter()
            .map(|format| ExportSetting {
                format: (*format).into(),
                scale: None,
                width: None,
                height: None,
                file_name: format!("frame.{format}"),
                svg_id_attribute: false,
            })
            .collect(),
    }
}

async fn exercise(spec: Option<ExportSpec>, delay_publication: bool) {
    let (root, store, snapshot) = fixture();
    let mut config = Config::for_backend(root.path().into(), 0, RenderBackend::Rust);
    config.node = "absent-node".into();
    let config = Arc::new(config);
    let (doorbell, _rings) = mpsc::channel(1);
    // Drive the actual runner explicitly, so the publication lock is deterministic.
    let jobs = Arc::new(Jobs {
        table: Mutex::new(Table::default()),
        doorbell,
        config: config.clone(),
        done: watch::channel(0).0,
    });
    let request = SceneRequest {
        frame_id: "frame".into(),
        revision: 1,
        width: 10.0,
        height: 10.0,
        scale: 0.3,
        theme: "default".into(),
        state: "default".into(),
    };
    let export = spec.is_some();
    let queued = if let Some(spec) = spec {
        jobs.export(snapshot, request, spec).await.unwrap()
    } else {
        jobs.enqueue(snapshot, request).await.unwrap()
    };
    let id = queued["jobId"].as_str().unwrap().to_owned();
    let work = jobs.table.lock().await.queue.pop().unwrap();
    let started = Instant::now();
    let deadline = started + Duration::from_secs(2);
    let result = run(&config, &store, &mut None, &work, deadline)
        .await
        .unwrap();
    assert!(
        result.1.is_ok(),
        "renderer must finish before controlled publication delay"
    );
    let guard = jobs.table.lock().await;
    let (events, _) = broadcast::channel(8);
    let (owner, finish_id) = (jobs.clone(), id.clone());
    let finish = tokio::spawn(async move {
        owner
            .finish(finish_id, Ok(result), started.elapsed(), deadline, &events)
            .await;
    });
    if delay_publication {
        tokio::time::sleep_until(deadline.into()).await;
        assert!(
            !finish.is_finished(),
            "publication is held by the table lock"
        );
    }
    drop(guard);
    finish.await.unwrap();
    let job = jobs
        .wait(&id, Duration::from_secs(1), pending())
        .await
        .unwrap();
    if delay_publication {
        assert_eq!(job["error"]["code"], "JOB_TIMEOUT", "{job}");
        assert!(job.get("imageUrl").is_none() && job.get("files").is_none());
        assert_eq!(jobs.image(&id).await.unwrap_err().0.code, "IMAGE_NOT_READY");
        assert!(jobs.file(&id, 0).await.is_err());
        assert_eq!(job["status"], if export { "failed" } else { "ready" });
        if !export {
            assert_eq!(job["scene"]["nodes"][0]["id"], "frame");
        }
    } else {
        assert!(job.get("error").is_none(), "{job}");
        assert_eq!(job["status"], "ready");
        if export {
            assert!(!jobs.file(&id, 0).await.unwrap().bytes.is_empty());
        } else {
            assert!(!jobs.image(&id).await.unwrap().is_empty());
            assert_eq!(job["image"]["provenance"]["scale"], 0.3);
        }
    }
}

#[tokio::test]
async fn completed_frame_and_all_export_files_expire_while_waiting_for_publication_lock() {
    exercise(None, true).await;
    exercise(Some(spec(&["png", "svg"])), true).await;
    exercise(Some(spec(&["svg"])), true).await;
}

#[tokio::test]
async fn completed_frame_and_exports_publish_before_the_deadline() {
    exercise(None, false).await;
    exercise(Some(spec(&["png", "svg"])), false).await;
}
