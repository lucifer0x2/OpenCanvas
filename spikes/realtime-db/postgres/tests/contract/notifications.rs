use super::support::*;
use crate::{batch, db, events, App};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

async fn listener_pid(url: &str, different_from: Option<i32>) -> i32 {
    let client = db::connect(url).await.unwrap();
    let name = format!("oc_spike_listener:{}", std::process::id());
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            let rows = client.query("SELECT pid FROM pg_stat_activity WHERE application_name=$1 AND state='idle' AND query='LISTEN oc_spike_changes'", &[&name]).await.unwrap();
            if let Some(pid) = rows.iter().map(|r| r.get::<_, i32>(0)).find(|p| Some(*p) != different_from) { break pid; }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await.expect("dedicated listener must establish/re-establish LISTEN")
}

#[tokio::test]
async fn notifications_are_post_commit_dry_run_silent_and_listener_recovers() {
    let (url, id) = project("listener").await;
    seed(&url, &id).await;
    let app = Arc::new(App {
        cache: None,
        url: url.clone(),
        wake: tokio::sync::Notify::new(),
    });
    let listener = tokio::spawn(events::listen(app.clone()));
    let first = listener_pid(&url, None).await;
    let wake = app.wake.notified();
    tokio::pin!(wake);
    wake.as_mut().enable();
    let mut dry = request(&id, "dry", 1, vec![update("root", 2)]);
    dry.dry_run = true;
    batch::execute(&url, &dry).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(350), wake.as_mut())
            .await
            .is_err(),
        "dry run must emit no PostgreSQL notification"
    );
    dry.dry_run = false;
    batch::execute(&url, &dry).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), wake.as_mut())
        .await
        .expect("LISTEN must receive committed batch notification");
    assert_eq!(db::snapshot(&url, &id).await.unwrap().revision, 2);
    let client = db::connect(&url).await.unwrap();
    assert!(client
        .query_one("SELECT pg_terminate_backend($1)", &[&first])
        .await
        .unwrap()
        .get::<_, bool>(0));
    let second = listener_pid(&url, Some(first)).await;
    assert_ne!(first, second);
    let wake = app.wake.notified();
    tokio::pin!(wake);
    wake.as_mut().enable();
    batch::execute(
        &url,
        &request(&id, "after_reconnect", 2, vec![update("root", 3)]),
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(2), wake)
        .await
        .expect("reconnected listener must receive new commits");
    assert_eq!(
        events::changes(&url, &id, 1).await.unwrap()["commits"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    listener.abort();
    let _ = listener.await;
    clean(&url, &id).await;
}

async fn open_stream(
    app: Arc<App>,
    id: &str,
    since: u64,
) -> (TcpStream, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let id = id.to_owned();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let _ = events::stream(&mut socket, &app, &id, since).await;
    });
    (TcpStream::connect(address).await.unwrap(), task)
}

async fn read_until(stream: &mut TcpStream, text: &mut String, needle: &str) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !text.contains(needle) {
            let mut bytes = [0; 4096];
            let count = stream.read(&mut bytes).await.unwrap();
            assert!(count > 0, "stream closed before expected event");
            text.push_str(std::str::from_utf8(&bytes[..count]).unwrap());
        }
    })
    .await
    .expect("durable polling catch-up must make progress without a listener");
}

#[tokio::test]
async fn polling_and_reconnected_stream_catch_up_without_any_notify_listener() {
    let (url, id) = project("poll").await;
    seed(&url, &id).await;
    // Deliberately never run events::listen: commits have no process-local wake.
    let app = Arc::new(App {
        cache: None,
        url: url.clone(),
        wake: tokio::sync::Notify::new(),
    });
    let (mut stream, task) = open_stream(app.clone(), &id, 1).await;
    let mut text = String::new();
    read_until(&mut stream, &mut text, ": heartbeat").await;
    batch::execute(
        &url,
        &request(&id, "offline_notify", 1, vec![update("root", 2)]),
    )
    .await
    .unwrap();
    read_until(&mut stream, &mut text, &format!("id: {id}:2\n")).await;
    assert_eq!(text.matches("\ndata: ").count(), 1);
    stream.shutdown().await.unwrap();
    task.abort();
    let _ = task.await;
    for revision in 3..=4 {
        batch::execute(
            &url,
            &request(
                &id,
                &format!("disconnected_{revision}"),
                revision - 1,
                vec![update("paired", revision)],
            ),
        )
        .await
        .unwrap();
    }
    let (mut resumed, task) = open_stream(app, &id, 2).await;
    let mut replay = String::new();
    read_until(&mut resumed, &mut replay, &format!("id: {id}:4\n")).await;
    let ids: Vec<_> = replay
        .lines()
        .filter(|line| line.starts_with("id: "))
        .collect();
    assert_eq!(ids, [format!("id: {id}:3"), format!("id: {id}:4")]);
    task.abort();
    let _ = task.await;
    clean(&url, &id).await;
}

#[tokio::test]
async fn invalid_future_stream_cursor_gets_reset_then_closes() {
    let (url, id) = project("stream_cursor").await;
    let app = Arc::new(App {
        cache: None,
        url: url.clone(),
        wake: tokio::sync::Notify::new(),
    });
    let (mut stream, task) = open_stream(app, &id, 1).await;
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), stream.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(response.contains("event: reset_required\n"));
    assert!(response.contains("\"reason\":\"invalid_cursor\""));
    assert!(response.contains("\"currentRevision\":0"));
    task.await.unwrap();
    clean(&url, &id).await;
}
