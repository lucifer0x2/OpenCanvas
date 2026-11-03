use crate::{db, App};
use oc_spike_core::DomainError;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::{io::AsyncWriteExt, net::TcpStream};

pub async fn changes(url: &str, id: &str, since: u64) -> Result<Value, DomainError> {
    let mut client = db::connect(url).await?;
    let tx = client
        .build_transaction()
        .isolation_level(tokio_postgres::IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await
        .map_err(db::internal)?;
    let revision = db::revision(&tx, id).await?;
    if since > revision {
        return Err(cursor_error(
            "INVALID_CURSOR",
            "Cursor exceeds current revision",
            revision,
        ));
    }
    let since = i64::try_from(since).map_err(db::internal)?;
    let rows = tx.query("SELECT revision,changed_ids FROM commits WHERE project_id=$1 AND revision>$2 ORDER BY revision", &[&id,&since]).await.map_err(db::internal)?;
    let commits: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({"revision":r.get::<_,i64>(0),"changedIds":r.get::<_,Value>(1)}))
        .collect();
    if commits.len() as u64 != revision - since as u64
        || commits.iter().enumerate().any(|(index, commit)| {
            commit["revision"].as_u64() != Some(since as u64 + index as u64 + 1)
        })
    {
        return Err(cursor_error(
            "REPLAY_GAP",
            "Durable commit history is incomplete",
            revision,
        ));
    }
    tx.commit().await.map_err(db::internal)?;
    Ok(json!({"projectId":id,"currentRevision":revision,"commits":commits}))
}

fn cursor_error(code: &str, message: &str, revision: u64) -> DomainError {
    DomainError {
        code: code.into(),
        message: message.into(),
        current_revision: Some(revision),
    }
}

pub async fn stream(
    socket: &mut TcpStream,
    app: &App,
    id: &str,
    mut since: u64,
) -> std::io::Result<()> {
    socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n: connected\n\n").await?;
    loop {
        // Enable notification before catch-up so a notification cannot be lost in the gap.
        let wake = app.wake.notified();
        tokio::pin!(wake);
        wake.as_mut().enable();
        match changes(&app.url, id, since).await {
            Ok(value) => {
                for commit in value["commits"].as_array().unwrap() {
                    let revision = commit["revision"].as_u64().unwrap();
                    let data = json!({"projectId":id,"revision":revision,"changedIds":commit["changedIds"]});
                    socket
                        .write_all(format!("id: {id}:{revision}\ndata: {data}\n\n").as_bytes())
                        .await?;
                    since = revision;
                }
            }
            Err(error) if ["INVALID_CURSOR", "REPLAY_GAP"].contains(&error.code.as_str()) => {
                let reason = if error.code == "INVALID_CURSOR" {
                    "invalid_cursor"
                } else {
                    "replay_gap"
                };
                let data = json!({"projectId":id,"currentRevision":error.current_revision,"reason":reason});
                socket
                    .write_all(format!("event: reset_required\ndata: {data}\n\n").as_bytes())
                    .await?;
                return Ok(());
            }
            Err(error) => eprintln!("SSE catch-up: {}", error.message),
        }
        tokio::select! { _ = wake => {}, _ = tokio::time::sleep(Duration::from_millis(250)) => {} }
        // Heartbeat also detects disconnected consumers without waiting for writes.
        socket.write_all(b": heartbeat\n\n").await?;
    }
}

pub async fn listen(app: Arc<App>) {
    loop {
        if let Err(error) = listen_once(&app).await {
            eprintln!("LISTEN reconnect: {error}");
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn listen_once(app: &Arc<App>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config: tokio_postgres::Config = app.url.parse()?;
    // Distinguishes this dedicated connection from pool traffic during recovery
    // tests and operational diagnosis. No credentials enter application_name.
    config.application_name(format!("oc_spike_listener:{}", std::process::id()));
    let (client, mut connection) = config.connect(tokio_postgres::NoTls).await?;
    let wake_app = app.clone();
    let driver = tokio::spawn(async move {
        loop {
            match std::future::poll_fn(|cx| connection.poll_message(cx)).await {
                Some(Ok(tokio_postgres::AsyncMessage::Notification(_))) => {
                    wake_app.wake.notify_waiters()
                }
                Some(Ok(_)) => {}
                Some(Err(error)) => return Err(error),
                None => return Ok(()),
            }
        }
    });
    if let Err(error) = client.batch_execute("LISTEN oc_spike_changes").await {
        driver.abort();
        let _ = driver.await;
        return Err(Box::new(error));
    }
    app.wake.notify_waiters();
    driver.await??;
    Ok(())
}
