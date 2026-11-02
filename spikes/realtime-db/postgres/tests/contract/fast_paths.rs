use super::support::*;
use crate::{batch, db, http, App};
use oc_spike_core::{BatchRequest, DomainError};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

async fn bounded_batch(url: &str, request: &BatchRequest) -> Result<Value, DomainError> {
    tokio::time::timeout(Duration::from_secs(2), batch::execute(url, request))
        .await
        .expect("fast path must finish while nodes remain ACCESS EXCLUSIVE locked")
}

async fn open(url: &str, path: &str) -> (TcpStream, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Arc::new(App {
        cache: None,
        url: url.into(),
        wake: tokio::sync::Notify::new(),
    });
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        http::serve(socket, app).await.unwrap();
    });
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket
        .write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
        .await
        .unwrap();
    (socket, server)
}

async fn get(url: &str, path: &str) -> (u16, Value) {
    let (mut socket, server) = open(url, path).await;
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), socket.read_to_string(&mut response))
        .await
        .expect("revision HTTP route must not read nodes")
        .unwrap();
    server.await.unwrap();
    let (headers, body) = response.split_once("\r\n\r\n").unwrap();
    let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, serde_json::from_str(body).unwrap())
}

async fn sse_connects_without_nodes(url: &str, id: &str) {
    let (mut socket, server) = open(url, &format!("/events/{id}?sinceRevision=1")).await;
    let mut header = Vec::new();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            header.push(socket.read_u8().await.unwrap());
            if header.ends_with(b"\r\n\r\n") {
                break;
            }
        }
    })
    .await
    .expect("SSE initial project existence check must not read nodes");
    assert!(header.starts_with(b"HTTP/1.1 200 "));
    drop(socket);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn stale_replay_collision_and_revision_read_do_not_touch_nodes() {
    let (url, id) = project("fast_paths").await;
    seed(&url, &id).await;
    let mut locker = db::connect(&url).await.unwrap();
    let pid: i32 = locker
        .query_one("SELECT pg_backend_pid()", &[])
        .await
        .unwrap()
        .get(0);
    let tx = locker.transaction().await.unwrap();
    tx.batch_execute("LOCK TABLE nodes IN ACCESS EXCLUSIVE MODE")
        .await
        .unwrap();

    let stale = request(&id, "stale", 0, vec![update("root", 8)]);
    let error = bounded_batch(&url, &stale).await.unwrap_err();
    assert_eq!(error.code, "REVISION_CONFLICT");
    assert_eq!(error.message, "Expected revision does not match");
    assert_eq!(error.current_revision, Some(1));
    let original = request(&id, "seed", 0, vec![page("root"), page("paired")]);
    let replay = bounded_batch(&url, &original).await.unwrap();
    assert_eq!(replay["committedRevision"], 1);
    assert_eq!(replay["replayed"], true);
    let mut collision = original.clone();
    collision.operations = vec![update("root", 9)];
    assert_eq!(
        bounded_batch(&url, &collision).await.unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    collision.dry_run = true;
    assert_eq!(
        bounded_batch(&url, &collision).await.unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    for mut dry in [original, stale] {
        dry.dry_run = true;
        let error = bounded_batch(&url, &dry).await.unwrap_err();
        assert_eq!(error.code, "REVISION_CONFLICT");
        assert_eq!(error.current_revision, Some(1));
    }
    let mut invalid = request(&id, "seed", 0, vec![]);
    assert_eq!(
        bounded_batch(&url, &invalid).await.unwrap_err().code,
        "VALIDATION_FAILED"
    );
    invalid.operations = vec![update("root", 9)];
    invalid.project_id = format!("missing_{id}");
    assert_eq!(
        bounded_batch(&url, &invalid).await.unwrap_err().code,
        "PROJECT_NOT_FOUND"
    );
    assert_eq!(
        get(&url, &format!("/projects/{id}/revision")).await,
        (200, json!({"projectId":id,"revision":1}))
    );
    sse_connects_without_nodes(&url, &id).await;

    // Positive control: a fresh valid batch must actually wait for the nodes lock.
    let fresh_url = url.clone();
    let fresh = request(&id, "fresh", 1, vec![update("root", 2)]);
    let blocked = tokio::spawn(async move { batch::execute(&fresh_url, &fresh).await });
    let observer = db::connect(&url).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let waiting: i64 = observer
                .query_one(
                    "SELECT count(*) FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))",
                    &[&pid],
                )
                .await
                .unwrap()
                .get(0);
            if waiting > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("fresh write must reach the nodes table lock");
    assert!(!blocked.is_finished());
    tx.commit().await.unwrap();
    assert_eq!(blocked.await.unwrap().unwrap()["committedRevision"], 2);
    assert_eq!(counts(&url, &id).await, [2, 2, 2, 2]);
    assert_eq!(
        get(&url, &format!("/projects/{id}/revision")).await,
        (200, json!({"projectId":id,"revision":2}))
    );
    clean(&url, &id).await;
}

#[tokio::test]
async fn revision_http_route_handles_missing_malformed_and_snapshot_paths() {
    let (url, id) = project("revision_http").await;
    assert_eq!(
        get(&url, &format!("/projects/{id}/revision")).await,
        (200, json!({"projectId":id,"revision":0}))
    );
    let (status, body) = get(&url, &format!("/projects/missing_{id}/revision")).await;
    assert_eq!(status, 404);
    assert_eq!(body["error"]["code"], "PROJECT_NOT_FOUND");
    for path in [
        "/projects//revision",
        "/projects/a/b/revision",
        "/projects/a%20b/revision",
    ] {
        let (status, body) = get(&url, path).await;
        assert_eq!(status, 400);
        assert_eq!(body["error"]["code"], "VALIDATION_FAILED");
    }
    let (status, snapshot) = get(&url, &format!("/projects/{id}")).await;
    assert_eq!(status, 200);
    assert_eq!(snapshot["projectId"], id);
    assert_eq!(snapshot["revision"], 0);
    assert_eq!(snapshot["nodes"], json!([]));
    // The literal project ID "revision" remains a snapshot route, without slicing panic.
    let (status, body) = get(&url, "/projects/revision").await;
    assert!([200, 404].contains(&status));
    if status == 200 {
        assert!(body["nodes"].is_array());
    } else {
        assert_eq!(body["error"]["code"], "PROJECT_NOT_FOUND");
    }
    clean(&url, &id).await;
}
