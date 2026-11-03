#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use std::{fs, time::Duration};
use support::{design, Server};

fn args(project: &str, revision: u64, key: &str) -> Value {
    json!({"projectId":project,"expectedRevision":revision,"idempotencyKey":key,"path":"ops.json"})
}
fn write(s: &Server, operations: &Value) {
    fs::write(
        s.root.path().join("test-project/inbox/ops.json"),
        serde_json::to_vec(operations).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn file_and_inline_share_atomic_commit_replay_conflict_and_validation() {
    let s = Server::start().await;
    let project = s.project().await;
    write(&s, &design());
    let request = args(&project, 0, "first");
    let mut preview = request.clone();
    preview["dryRun"] = json!(true);
    let result = s.call("apply_batch_file", preview).await;
    assert_eq!(result["baseRevision"], 0, "{result}");
    assert_eq!(result["dryRun"], true);
    let committed = s.call("apply_batch_file", request.clone()).await;
    assert_eq!(committed["committedRevision"], 1, "{committed}");
    assert_eq!(result["changedIds"], committed["changedIds"]);
    let replay = s.batch(&project, 0, "first", design()).await;
    assert_eq!(replay["replayed"], true, "{replay}");
    write(
        &s,
        &json!([{"op":"update_node","id":"text","props":{"text":"changed"}}]),
    );
    let reused = s.call("apply_batch_file", request).await;
    assert_eq!(
        reused["error"]["code"], "IDEMPOTENCY_KEY_REUSED",
        "{reused}"
    );
    let stale = s.call("apply_batch_file", args(&project, 0, "stale")).await;
    assert_eq!(stale["error"]["code"], "REVISION_CONFLICT");
    assert_eq!(stale["error"]["changedSince"]["toRevision"], 1);
    let invalid = json!([
        {"op":"update_node","id":"text","props":{"text":"must roll back"}},
        {"op":"update_node","id":"frame","props":{"styleId":"missing"}}
    ]);
    write(&s, &invalid);
    let error = s
        .call("apply_batch_file", args(&project, 1, "invalid"))
        .await;
    assert_eq!(error["error"]["code"], "VALIDATION_FAILED", "{error}");
    let snap = s.get(&format!("/api/v1/projects/{project}/snapshot")).await;
    assert_eq!(snap["revision"], 1);
    assert_eq!(
        snap["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "text")
            .unwrap()["props"]["text"],
        "Tiếng Việt OpenCanvas"
    );
}

#[tokio::test]
async fn file_original_json_obeys_the_published_schema_and_paths_are_private() {
    let s = Server::start().await;
    let project = s.project().await;
    // parentId is required by the public schema even though serde can default Option.
    for operations in [
        json!([{"op":"create_node","id":"page","kind":"page","props":{}}]),
        json!([{"op":"create_node","id":"page","kind":"page","parentId":null,"props":{},"unknown":true}]),
        json!([{"op":"update_text","nodeId":"text","props":{"content":"bad"}}]),
    ] {
        write(&s, &operations);
        let error = s.call("apply_batch_file", args(&project, 0, "bad")).await;
        assert_eq!(error["error"]["code"], "INVALID_REQUEST", "{error}");
    }
    for path in ["../db.sql", "/etc/passwd", "ops.js"] {
        let mut request = args(&project, 0, "bad");
        request["path"] = json!(path);
        let result = s.call("apply_batch_file", request).await;
        assert!(result["error"].is_object());
        assert!(!result.to_string().contains(s.root.path().to_str().unwrap()));
    }
    let mut bad_dry = args(&project, 0, "bad");
    bad_dry["dryRun"] = json!("true");
    assert_eq!(
        s.call("apply_batch_file", bad_dry).await["error"]["code"],
        "INVALID_REQUEST"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn dry_run_has_no_durable_event_asset_or_prerender_effects() {
    use opencanvas_storage::BatchRequest;
    use std::os::unix::fs::PermissionsExt;
    let marker = tempfile::tempdir().unwrap();
    let executable = marker.path().join("worker");
    let invoked = marker.path().join("invoked");
    fs::write(
        &executable,
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", invoked.display()),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let s = Server::configured(|c| c.node = executable.to_str().unwrap().into()).await;
    let project = s.project().await;
    // Seed through storage, so a prior domain prerender cannot pollute this proof.
    let seeded = project.clone();
    s.state
        .db(move |store| {
            store.open_project(&seeded)?.apply_batch(BatchRequest {
                project_id: seeded,
                expected_revision: 0,
                idempotency_key: "seed".into(),
                operations: serde_json::from_value(design()).unwrap(),
            })
        })
        .await
        .unwrap();
    let mut events = s.state.events.subscribe();
    let operations = json!([{"op":"update_node","id":"text","props":{"text":"Preview"}}]);
    write(&s, &operations);
    let mut file = args(&project, 1, "preview");
    file["dryRun"] = json!(true);
    let from_file = s.call("apply_batch_file", file).await;
    let inline = json!({"projectId":project,"expectedRevision":1,"idempotencyKey":"preview","operations":operations,"dryRun":true});
    let result = s.call("apply_batch", inline).await;
    assert_eq!(from_file, result, "{result}");
    assert_eq!(result["changedIds"], json!(["text"]));
    assert_eq!(result["diagnostics"], json!([]));
    assert!(
        tokio::time::timeout(Duration::from_millis(150), events.recv())
            .await
            .is_err()
    );
    assert!(!invoked.exists(), "dry run started render worker");
    let connection = rusqlite::Connection::open(s.root.path().join("test-project/db.sql")).unwrap();
    for table in ["commits", "idempotency"] {
        assert_eq!(
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    assert_eq!(
        fs::read_dir(s.root.path().join("test-project/assets"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        s.state
            .store
            .open_project(&project)
            .unwrap()
            .snapshot()
            .unwrap()
            .revision,
        1
    );
    // A real commit consumes that same preview key and proves the worker probe works.
    let commit = s
        .call("apply_batch_file", args(&project, 1, "preview"))
        .await;
    assert_eq!(commit["committedRevision"], 2, "{commit}");
    assert_eq!(events.recv().await.unwrap()["event"], "commit");
    for _ in 0..50 {
        if invoked.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        invoked.exists(),
        "real commit did not exercise the worker probe"
    );
}
