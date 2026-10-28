//! Task08: workspace hints (`project_created`, `project_activity`) on the project stream and on
//! `GET /api/v1/events`, without durable replay for another project's commits.
mod support;
use futures_util::{stream::BoxStream, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use support::Server;

type Body = BoxStream<'static, reqwest::Result<axum::body::Bytes>>;

/// Opens an SSE path; the server subscribed to the broadcast before the headers arrived.
async fn open(s: &Server, path: &str) -> Body {
    let response = s
        .client
        .get(format!("{}{path}", s.url))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success(), "{}", response.status());
    let kind = response.headers()["content-type"].to_str().unwrap();
    assert!(kind.starts_with("text/event-stream"), "{kind}");
    response.bytes_stream().boxed()
}

/// Reads until `text` contains `needle`, appending to `text` (4 s at most).
async fn until(stream: &mut Body, text: &mut String, needle: &str) {
    let read = tokio::time::timeout(Duration::from_secs(4), async {
        while !text.contains(needle) {
            match stream.next().await {
                Some(Ok(chunk)) => text.push_str(&String::from_utf8_lossy(&chunk)),
                _ => break,
            }
        }
    })
    .await;
    assert!(read.is_ok() && text.contains(needle), "{needle} in {text}");
}

/// Event names in order, skipping heartbeat comments.
fn names(text: &str) -> Vec<&str> {
    text.lines()
        .filter_map(|line| line.strip_prefix("event: "))
        .collect()
}

async fn create(s: &Server, slug: &str) -> String {
    let created = s
        .call("create_project", json!({"slug":slug,"name":slug}))
        .await;
    created["projectId"].as_str().unwrap().into()
}

fn page() -> Value {
    json!([{"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"name":"Home"}}])
}

#[tokio::test]
async fn project_stream_forwards_hints_without_replaying_the_log() {
    let s = Server::start().await;
    let a = create(&s, "alpha").await;
    s.batch(&a, 0, "a1", page()).await;
    let mut stream = open(&s, &format!("/api/v1/projects/{a}/events?cursor={a}:1")).await;
    // Trap: any durable replay of A from now on fails and ends the stream with a reset.
    let db = rusqlite::Connection::open(s.root.path().join("alpha/db.sql")).unwrap();
    db.execute("ALTER TABLE commits RENAME TO commits_hidden", [])
        .unwrap();
    drop(db);

    let mut text = String::new();
    let b = create(&s, "beta").await;
    until(&mut stream, &mut text, "event: project_created").await;
    let b1 = s.batch(&b, 0, "b1", page()).await;
    assert_eq!(b1["committedRevision"], 1, "{b1}");
    until(&mut stream, &mut text, "event: project_activity").await;
    let scene =
        json!({"event":"scene_ready","projectId":a,"revision":1,"sceneKey":"k","jobId":"j"});
    s.state.events.send(scene).unwrap();
    until(&mut stream, &mut text, "event: scene_ready").await;
    assert_eq!(
        names(&text),
        ["project_created", "project_activity", "scene_ready"]
    );
    assert!(text.contains(&format!(
        r#"data: {{"name":"beta","projectId":"{b}","revision":0,"slug":"beta"}}"#
    )));
    assert!(text.contains(&format!(r#"data: {{"projectId":"{b}","revision":1}}"#)));
    assert!(!text.contains("id: "), "hints carry no SSE id: {text}");

    // The trap is armed: a commit of A itself does replay, which proves the hints above did not.
    let own = json!({"event":"commit","projectId":a,"revision":2,"changedIds":[]});
    s.state.events.send(own).unwrap();
    until(&mut stream, &mut text, "event: reset_required").await;
    assert!(text.contains("\"reason\":\"replay_unavailable\""), "{text}");
    assert!(!text.contains("event: commit"), "{text}");
}

#[tokio::test]
async fn workspace_stream_reports_created_projects_and_activity() {
    let s = Server::start().await;
    let mut stream = open(&s, "/api/v1/events").await;
    let mut text = String::new();
    let p = create(&s, "first").await;
    until(&mut stream, &mut text, "event: project_created").await;
    s.batch(&p, 0, "p1", page()).await;
    until(&mut stream, &mut text, "event: project_activity").await;
    assert_eq!(names(&text), ["project_created", "project_activity"]);
    assert!(text.contains(&format!(r#""projectId":"{p}""#)), "{text}");
    assert!(!text.contains("id: "), "{text}");

    let bad = s.get("/api/v1/events?cursor=x").await;
    assert_eq!(bad["error"]["code"], "INVALID_REQUEST");
    let rejected = s
        .client
        .get(format!("{}/api/v1/events", s.url))
        .header("host", "attacker.test")
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), 403);

    s.state.begin_shutdown();
    let ended = tokio::time::timeout(Duration::from_secs(4), async {
        while let Some(chunk) = stream.next().await {
            if chunk.is_err() {
                break;
            }
        }
    })
    .await;
    assert!(ended.is_ok(), "the workspace stream must end on shutdown");
}

#[tokio::test]
async fn failed_creation_sends_no_hint() {
    let s = Server::start().await;
    create(&s, "taken").await;
    let mut events = s.state.events.subscribe();
    let duplicate = s
        .call("create_project", json!({"slug":"taken","name":"again"}))
        .await;
    assert!(duplicate.get("error").is_some(), "{duplicate}");
    assert!(events.try_recv().is_err(), "no event after a failed create");
}
