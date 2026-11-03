//! The project event stream learns the canonical ID and current revision from project metadata.
//! It never loads a full snapshot: on PostgreSQL every commit invalidates the snapshot cache, so a
//! snapshot read per connected client per commit would be a full entity read each.
#[allow(dead_code)] // Shared fixture; this file uses only part of it.
mod storage_backend_support;
use futures_util::{stream::BoxStream, StreamExt};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use storage_backend_support::{memory::MemoryBackend, Server};

type Body = BoxStream<'static, reqwest::Result<axum::body::Bytes>>;

async fn open(s: &Server, path: &str) -> Body {
    let response = s
        .client
        .get(format!("{}{path}", s.url))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success(), "{}", response.status());
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

async fn page(s: &Server, project: &str, revision: u64, id: &str) -> Value {
    let args = json!({"projectId":project,"expectedRevision":revision,"idempotencyKey":id,
        "operations":[{"op":"create_node","id":id,"kind":"page","parentId":null,"props":{"name":id}}]});
    s.call("apply_batch", args).await
}

fn count(calls: &[&str], name: &str) -> usize {
    calls.iter().filter(|c| **c == name).count()
}

#[tokio::test]
async fn project_stream_reads_metadata_not_snapshots() {
    let backend = Arc::new(MemoryBackend::default());
    let s = Server::start(backend.clone()).await;
    let created = s
        .call("create_project", json!({"slug":"meta-only","name":"Meta"}))
        .await;
    let id = created["projectId"].as_str().unwrap().to_owned();
    assert_eq!(page(&s, &id, 0, "first").await["committedRevision"], 1);

    let before = backend.calls().len();
    // Opened by slug; the cursor names the canonical ID, so the slug was normalized.
    let mut stream = open(
        &s,
        &format!("/api/v1/projects/meta-only/events?cursor={id}:0"),
    )
    .await;
    let mut text = String::new();
    until(&mut stream, &mut text, &format!("id: {id}:1")).await;
    assert_eq!(page(&s, &id, 1, "second").await["committedRevision"], 2);
    until(&mut stream, &mut text, &format!("id: {id}:2")).await;

    // Since the stream opened, each mutation read one snapshot for prerender targets; the
    // stream itself (open, replay, wake-up) read none.
    let calls = backend.calls().split_off(before);
    assert_eq!(
        count(&calls, "snapshot"),
        count(&calls, "apply"),
        "{calls:?}"
    );
    assert!(count(&calls, "meta") >= 2, "{calls:?}");

    // A cursor ahead of the project resets with the metadata revision.
    let mut ahead = open(&s, &format!("/api/v1/projects/{id}/events?cursor={id}:9")).await;
    let mut text = String::new();
    until(&mut ahead, &mut text, "invalid_cursor").await;
    assert!(text.contains("\"currentRevision\":2"), "{text}");

    let missing = s
        .client
        .get(format!("{}/api/v1/projects/absent/events", s.url))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
    let body: Value = missing.json().await.unwrap();
    assert_eq!(body["error"]["code"], "PROJECT_NOT_FOUND");
}
