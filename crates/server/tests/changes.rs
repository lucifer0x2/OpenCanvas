//! `get_changes`: commits after a revision from the durable log, grouped against the current
//! snapshot, paged by whole commits within `limit` and `budget`.
#[allow(dead_code)] // `get` and `wait_job` are unused here.
mod support;
use serde_json::{json, Value};
use support::{design, Server};

const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=";

/// r1 design; r2 token + frame fill; r3 delete the text; r4 asset import. Returns (project, asset).
async fn history(s: &Server) -> (String, String) {
    let p = s.project().await;
    s.batch(&p, 0, "r1", design()).await;
    let r2 = json!([
        {"op":"upsert_token","token":{"id":"accent","name":"Accent","type":"color","value":"#ff0000"}},
        {"op":"update_node","id":"frame","props":{"fill":"#000000"}}
    ]);
    s.batch(&p, 1, "r2", r2).await;
    s.batch(&p, 2, "r3", json!([{"op":"delete_subtree","id":"text"}]))
        .await;
    let import = s.call("import_asset", json!({"projectId":p,"expectedRevision":3,"idempotencyKey":"r4","mediaType":"image/png","base64":PNG})).await;
    assert_eq!(import["result"]["committedRevision"], 4, "{import}");
    (p, import["asset"]["id"].as_str().unwrap().into())
}

fn code(v: &Value) -> &str {
    v["error"]["code"].as_str().unwrap_or_default()
}

#[tokio::test]
async fn changes_are_grouped_against_the_current_snapshot() {
    let s = Server::start().await;
    let (p, asset) = history(&s).await;
    let all = s
        .call("get_changes", json!({"projectId":p,"sinceRevision":0}))
        .await;
    assert_eq!(
        (
            all["fromRevision"].as_u64(),
            all["currentRevision"].as_u64()
        ),
        (Some(0), Some(4))
    );
    assert_eq!(
        (all["truncated"].as_bool(), all["nextRevision"].as_u64()),
        (Some(false), Some(4))
    );
    let revisions: Vec<_> = all["commits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["revision"].as_u64().unwrap())
        .collect();
    assert_eq!(revisions, [1, 2, 3, 4]);
    assert_eq!(all["commits"][2]["changedIds"], json!(["text"]));
    assert!(
        all["commits"][0].get("operations").is_none(),
        "operations are opt-in"
    );
    let changed = &all["changed"];
    assert_eq!(changed["nodes"], json!(["frame", "page"]), "{changed}");
    assert_eq!(changed["tokens"], json!(["accent"]));
    assert_eq!(changed["styles"], json!([]));
    assert_eq!(changed["assets"], json!([asset]));
    assert_eq!(changed["removed"], json!(["text"]));
    assert_eq!(all["limits"], json!({"commits":100,"bytes":65536}));
    // A later cursor only sees later commits; the text was deleted at r3, so it is removed.
    let since2 = s
        .call("get_changes", json!({"projectId":p,"sinceRevision":2}))
        .await;
    assert_eq!(since2["changed"]["nodes"], json!([]));
    assert_eq!(since2["changed"]["removed"], json!(["text"]));
    // Caught up: nothing to report, the next cursor is the current revision.
    let none = s
        .call(
            "get_changes",
            json!({"projectId":"test-project","sinceRevision":4}),
        )
        .await;
    assert_eq!(
        (none["commits"].clone(), none["nextRevision"].as_u64()),
        (json!([]), Some(4))
    );
    assert_eq!(none["truncated"], false);
}

#[tokio::test]
async fn operations_are_opt_in_and_never_expose_storage_keys() {
    let s = Server::start().await;
    let (p, _) = history(&s).await;
    let ops = s
        .call(
            "get_changes",
            json!({"projectId":p,"sinceRevision":2,"includeOperations":true}),
        )
        .await;
    assert_eq!(
        ops["commits"][0]["operations"],
        json!([{"op":"delete_subtree","id":"text"}])
    );
    let import = &ops["commits"][1]["operations"][0];
    assert_eq!(import["op"], "import_asset", "{import}");
    assert!(import["asset"]["id"].is_string());
    assert!(import["asset"].get("relativeKey").is_none(), "{import}");
    assert!(!ops.to_string().contains("relativeKey"));
}

#[tokio::test]
async fn pages_follow_next_revision_within_limit_and_budget() {
    let s = Server::start().await;
    let (p, _) = history(&s).await;
    let first = s
        .call(
            "get_changes",
            json!({"projectId":p,"sinceRevision":0,"limit":1}),
        )
        .await;
    assert_eq!(
        (first["truncated"].as_bool(), first["nextRevision"].as_u64()),
        (Some(true), Some(1))
    );
    assert_eq!(first["commits"].as_array().unwrap().len(), 1);
    // Every budget yields pages within it that, followed by nextRevision, cover every commit.
    for budget in (1024..=2048).step_by(64) {
        let (mut since, mut seen) = (0, Vec::new());
        loop {
            let page = s
                .call("get_changes", json!({"projectId":p,"sinceRevision":since,"budget":budget,"includeOperations":true}))
                .await;
            assert!(page.get("error").is_none(), "{budget}: {page}");
            assert!(page.to_string().len() <= budget, "{budget}");
            for c in page["commits"].as_array().unwrap() {
                seen.push(c["revision"].as_u64().unwrap());
            }
            since = page["nextRevision"].as_u64().unwrap();
            if page["truncated"] == false {
                break;
            }
        }
        assert_eq!(seen, [1, 2, 3, 4], "{budget}");
    }
}

#[tokio::test]
async fn limits_and_cursors_fail_with_existing_codes() {
    let s = Server::start().await;
    let (p, _) = history(&s).await;
    let ahead = s
        .call("get_changes", json!({"projectId":p,"sinceRevision":5}))
        .await;
    assert_eq!(
        (code(&ahead), ahead["error"]["currentRevision"].as_u64()),
        ("RESET_REQUIRED", Some(4))
    );
    for bad in [
        json!({"projectId":p}),
        json!({"projectId":p,"sinceRevision":-1}),
        json!({"projectId":p,"sinceRevision":0,"limit":101}),
        json!({"projectId":p,"sinceRevision":0,"limit":0}),
        json!({"projectId":p,"sinceRevision":0,"budget":1023}),
        json!({"projectId":p,"sinceRevision":0,"budget":65537}),
        json!({"projectId":p,"sinceRevision":0,"includeOperations":"yes"}),
        json!({"projectId":p,"sinceRevision":0,"cursor":"x"}),
    ] {
        assert_eq!(
            code(&s.call("get_changes", bad.clone()).await),
            "INVALID_REQUEST",
            "{bad}"
        );
    }
    let missing = s
        .call("get_changes", json!({"projectId":"nope","sinceRevision":0}))
        .await;
    assert_eq!(code(&missing), "PROJECT_NOT_FOUND");
    // One commit larger than the budget cannot be split.
    let nodes: Vec<_> = (0..40).map(|i| json!({"op":"create_node","id":format!("n{i}"),"kind":"rectangle","parentId":"frame","props":{"name":"x".repeat(64),"width":1,"height":1}})).collect();
    s.batch(&p, 4, "big", json!(nodes)).await;
    let big = s
        .call(
            "get_changes",
            json!({"projectId":p,"sinceRevision":4,"budget":1024,"includeOperations":true}),
        )
        .await;
    assert_eq!(code(&big), "CONTEXT_ITEM_TOO_LARGE");
}

#[tokio::test]
async fn missing_history_requires_a_reset() {
    let s = Server::start().await;
    let (p, _) = history(&s).await;
    let db = rusqlite::Connection::open(s.root.path().join("test-project/db.sql")).unwrap();
    db.execute("DELETE FROM commits WHERE revision IN (2, 4)", [])
        .unwrap();
    drop(db);
    for since in [0, 1, 3] {
        let gap = s
            .call("get_changes", json!({"projectId":p,"sinceRevision":since}))
            .await;
        assert_eq!(
            (code(&gap), gap["error"]["currentRevision"].as_u64()),
            ("RESET_REQUIRED", Some(4)),
            "{since}"
        );
    }
    // A page that ends before the gap is still served.
    let before = s
        .call(
            "get_changes",
            json!({"projectId":p,"sinceRevision":0,"limit":1}),
        )
        .await;
    assert_eq!(before["nextRevision"], 1, "{before}");
}
