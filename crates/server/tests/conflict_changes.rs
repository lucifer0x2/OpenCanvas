//! `REVISION_CONFLICT` carries `changedSince`: the IDs changed after the loser's expectedRevision.
//! Code, message, retryable and `currentRevision` stay exactly as storage reports them.
#[allow(dead_code)] // `get` and `wait_job` are unused here.
mod support;
use serde_json::{json, Value};
use support::{design, Server};

const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=";

fn update(id: &str, text: &str) -> Value {
    json!([{"op":"update_node","id":id,"props":{"text":text}}])
}

#[tokio::test]
async fn a_lost_batch_learns_what_changed_and_rebases() {
    let s = Server::start().await;
    let p = s.project().await;
    s.batch(&p, 0, "initial", design()).await;
    let more = json!([{"op":"create_node","id":"other","kind":"text","parentId":"frame","props":{"text":"x","width":100,"height":"hug","fontId":"font_bundled","fontSize":12,"lineHeight":16}}]);
    s.batch(&p, 1, "winner-1", more).await;
    s.batch(&p, 2, "winner-2", update("other", "y")).await;
    let lost = s.batch(&p, 1, "loser-1", update("text", "mine")).await;
    let error = &lost["error"];
    assert_eq!(error["code"], "REVISION_CONFLICT");
    assert_eq!(error["currentRevision"], 3);
    assert_eq!(error["retryable"], false);
    assert_eq!(
        error["message"],
        "Read the current snapshot and retry with its revision"
    );
    assert_eq!(
        error["changedSince"],
        json!({"fromRevision":1,"toRevision":3,"changedIds":["other"],"truncated":false})
    );
    // Disjoint from the edited node: rebase with the new revision and a new key.
    let retried = s.batch(&p, 3, "loser-2", update("text", "mine")).await;
    assert_eq!(retried["committedRevision"], 4, "{retried}");
    // Asset imports lose the same way.
    let import = s.call("import_asset", json!({"projectId":p,"expectedRevision":3,"idempotencyKey":"img","mediaType":"image/png","base64":PNG})).await;
    assert_eq!(import["error"]["code"], "REVISION_CONFLICT", "{import}");
    assert_eq!(
        import["error"]["changedSince"],
        json!({"fromRevision":3,"toRevision":4,"changedIds":["text"],"truncated":false})
    );
}

#[tokio::test]
async fn changed_ids_are_capped_and_truncation_is_explicit() {
    let s = Server::start().await;
    let p = s.project().await;
    s.batch(&p, 0, "initial", design()).await;
    let nodes: Vec<_> = (0..250).map(|i| json!({"op":"create_node","id":format!("n{i:03}"),"kind":"rectangle","parentId":"frame","props":{"width":1,"height":1}})).collect();
    s.batch(&p, 1, "many", json!(nodes)).await;
    let lost = s.batch(&p, 1, "loser", update("text", "mine")).await;
    let since = &lost["error"]["changedSince"];
    assert_eq!(
        since["changedIds"].as_array().unwrap().len(),
        200,
        "{since}"
    );
    assert_eq!(since["truncated"], true);
    assert_eq!(lost["error"]["currentRevision"], 2);
}

#[tokio::test]
async fn conflicts_without_a_usable_range_stay_plain_or_truncated() {
    let s = Server::start().await;
    let p = s.project().await;
    s.batch(&p, 0, "initial", design()).await;
    s.batch(&p, 1, "second", update("text", "b")).await;
    // expectedRevision ahead of the project: nothing changed "since" it, so no detail.
    let ahead = s.batch(&p, 9, "ahead", update("text", "c")).await;
    assert_eq!(ahead["error"]["code"], "REVISION_CONFLICT");
    assert_eq!(ahead["error"]["currentRevision"], 2);
    assert!(ahead["error"].get("changedSince").is_none(), "{ahead}");
    // A missing log row cannot be reported as complete.
    let db = rusqlite::Connection::open(s.root.path().join("test-project/db.sql")).unwrap();
    db.execute("DELETE FROM commits WHERE revision=1", [])
        .unwrap();
    drop(db);
    let gap = s.batch(&p, 0, "gap", update("text", "d")).await;
    assert_eq!(
        gap["error"]["changedSince"],
        json!({"fromRevision":0,"toRevision":2,"changedIds":[],"truncated":true})
    );
    // Other errors are untouched.
    let invalid = s.batch(&p, 2, "invalid", update("missing", "e")).await;
    assert_eq!(invalid["error"]["code"], "VALIDATION_FAILED");
    assert!(invalid["error"].get("changedSince").is_none());
}
