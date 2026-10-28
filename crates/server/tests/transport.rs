mod support;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use support::{design, Server};

#[tokio::test]
async fn mcp_atomic_conflict_replay_snapshot_and_guards() {
    let s = Server::start().await;
    let tools = s.rpc("tools/list", json!({})).await;
    assert_eq!(tools["tools"].as_array().unwrap().len(), 15);
    let p = s.project().await;
    let first = s.batch(&p, 0, "initial", design()).await;
    assert_eq!(first["committedRevision"], 1, "{first}");
    let replay = s.batch(&p, 0, "initial", design()).await;
    assert_eq!(replay["committedRevision"], 1);
    assert_eq!(replay["replayed"], true);
    let reused = s
        .batch(
            &p,
            0,
            "initial",
            json!([{"op":"update_node","id":"text","props":{"text":"different"}}]),
        )
        .await;
    assert_eq!(reused["error"]["code"], "IDEMPOTENCY_KEY_REUSED");
    let a = s.batch(
        &p,
        1,
        "writer_a",
        json!([{"op":"update_node","id":"text","props":{"text":"A"}}]),
    );
    let b = s.batch(
        &p,
        1,
        "writer_b",
        json!([{"op":"update_node","id":"text","props":{"text":"B"}}]),
    );
    let (a, b) = tokio::join!(a, b);
    assert_eq!(
        [&a, &b]
            .iter()
            .filter(|v| v["committedRevision"] == 2)
            .count(),
        1
    );
    assert_eq!(
        [&a, &b]
            .iter()
            .filter(|v| v["error"]["code"] == "REVISION_CONFLICT")
            .count(),
        1
    );
    let bad = s
        .batch(
            &p,
            2,
            "rollback",
            json!([
                {"op":"update_node","id":"text","props":{"text":"partial"}},
                {"op":"update_node","id":"missing","props":{"text":"bad"}}
            ]),
        )
        .await;
    assert!(bad.get("error").is_some());
    let snap = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    assert_eq!(snap["revision"], 2);
    assert!(!snap["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["props"]["text"] == "partial"));
    for args in [
        json!({"projectId":p}),
        json!({"projectId":p,"nodeId":"frame","limit":201}),
        json!({"projectId":p,"nodeId":"frame","unexpected":true}),
    ] {
        assert_eq!(
            s.call("get_subtree", args).await["error"]["code"],
            "INVALID_REQUEST"
        );
    }
    let rejected = s
        .client
        .get(format!("{}/api/v1/projects", s.url))
        .header("host", "attacker.test")
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), 403);
    let rejected = s
        .client
        .get(format!("{}/api/v1/projects", s.url))
        .header("origin", "https://attacker.test")
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), 403);
    assert_eq!(
        s.get("/api/v1/projects?limit=bad").await["error"]["code"],
        "INVALID_REQUEST"
    );
    assert!(s.root.path().join("test-project/db.sql").is_file());
}

async fn events(s: &Server, path: &str, count: usize) -> String {
    let response = s
        .client
        .get(format!("{}{path}", s.url))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let mut stream = response.bytes_stream();
    tokio::time::timeout(Duration::from_secs(4), async {
        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            text.push_str(&String::from_utf8_lossy(&chunk.unwrap()));
            if text.matches("event: commit").count() >= count || text.contains("reset_required") {
                return text;
            }
        }
        text
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn sse_replays_durable_commits_and_resets_bad_cursors() {
    let s = Server::start().await;
    let p = s.project().await;
    s.batch(&p, 0, "initial", design()).await;
    s.batch(
        &p,
        1,
        "second",
        json!([{"op":"update_node","id":"text","props":{"text":"second"}}]),
    )
    .await;
    let text = events(&s, &format!("/api/v1/projects/{p}/events?cursor={p}:0"), 2).await;
    assert!(
        text.contains(&format!("id: {p}:1")) && text.contains(&format!("id: {p}:2")),
        "{text}"
    );
    for cursor in ["garbage".into(), "other:1".into(), format!("{p}:99")] {
        let text = events(
            &s,
            &format!("/api/v1/projects/{p}/events?cursor={cursor}"),
            1,
        )
        .await;
        assert!(
            text.contains("reset_required") && text.contains("\"currentRevision\":2"),
            "{text}"
        );
    }
    let path = format!("/api/v1/projects/{p}/events?cursor={p}:2");
    let response = s
        .client
        .get(format!("{}{path}", s.url))
        .send()
        .await
        .unwrap();
    let mut stream = response.bytes_stream();
    s.batch(
        &p,
        2,
        "race",
        json!([{"op":"update_node","id":"text","props":{"text":"race"}}]),
    )
    .await;
    let text = tokio::time::timeout(Duration::from_secs(4), async {
        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            text.push_str(&String::from_utf8_lossy(&chunk.unwrap()));
            if text.contains("event: commit") {
                break;
            }
        }
        text
    })
    .await
    .unwrap();
    assert!(text.contains(&format!("id: {p}:3")), "{text}");
    let value: Value = s
        .get(&format!("/api/v1/projects/{p}/events?unknown=x"))
        .await;
    assert_eq!(value["error"]["code"], "INVALID_REQUEST");
}

#[tokio::test]
async fn sse_durable_gap_requires_reset_at_actual_revision() {
    let s = Server::start().await;
    let p = s.project().await;
    s.batch(&p, 0, "initial", design()).await;
    s.batch(
        &p,
        1,
        "second",
        json!([{"op":"update_node","id":"text","props":{"text":"second"}}]),
    )
    .await;
    let db = rusqlite::Connection::open(s.root.path().join("test-project/db.sql")).unwrap();
    db.execute("DELETE FROM commits WHERE revision=1", [])
        .unwrap();
    drop(db);
    let text = events(&s, &format!("/api/v1/projects/{p}/events?cursor={p}:0"), 1).await;
    assert!(
        text.contains("reset_required") && text.contains("\"currentRevision\":2"),
        "{text}"
    );
    assert!(!text.contains("event: commit"), "{text}");
}

#[tokio::test]
async fn shutdown_ends_open_event_streams() {
    let s = Server::start().await;
    let p = s.project().await;
    s.batch(&p, 0, "initial", design()).await;
    let response = s
        .client
        .get(format!("{}/api/v1/projects/{p}/events", s.url))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let mut stream = response.bytes_stream();
    s.state.begin_shutdown();
    // A stream that ignores shutdown holds axum's graceful shutdown open for as long as its
    // client stays connected, which is what made Ctrl-C hang with the viewer open.
    let ended = tokio::time::timeout(Duration::from_secs(4), async {
        while let Some(chunk) = stream.next().await {
            if chunk.is_err() {
                break;
            }
        }
    })
    .await;
    assert!(ended.is_ok(), "an open event stream must end on shutdown");
}
