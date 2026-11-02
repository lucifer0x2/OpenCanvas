use crate::{
    batch, cache_metrics, compatibility, db, durable_http, events, hot_reads,
    transport::{read, send, send_bytes, send_error, Request},
    App,
};
use oc_spike_core::{BatchRequest, DomainError};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::{io::AsyncWriteExt, net::TcpStream};

pub async fn serve(mut socket: TcpStream, app: Arc<App>) -> std::io::Result<()> {
    let request = match read(&mut socket).await {
        Ok(value) => value,
        Err(error) => {
            send_error(&mut socket, error).await?;
            // An early size/header rejection may leave incoming body bytes. Closing
            // immediately can reset TCP and hide the already-written JSON response.
            // Drain into a sink with bounded time and constant memory after FIN.
            let _ = tokio::time::timeout(
                std::time::Duration::from_millis(250),
                tokio::io::copy(&mut socket, &mut tokio::io::sink()),
            )
            .await;
            return Ok(());
        }
    };
    let (path, query) = request.path.split_once('?').unwrap_or((&request.path, ""));
    if request.method == "GET" && path.starts_with("/events/") {
        let id = &path[8..];
        let since = since(query);
        return match since {
            Ok(since) => match db::current_revision(&app.url, id).await {
                Ok(_) => events::stream(&mut socket, &app, id, since).await,
                Err(error) => send_error(&mut socket, error).await,
            },
            Err(error) => send_error(&mut socket, error).await,
        };
    }
    if request.method == "GET" && path.starts_with("/projects/") && !path.ends_with("/revision") {
        return match hot_reads::snapshot(&app.url, &path[10..], app.cache.as_ref()).await {
            Ok(bytes) => send_bytes(&mut socket, 200, &bytes).await,
            Err(error) => send_error(&mut socket, error).await,
        };
    }
    match route(&app, &request).await {
        Ok(value) => {
            if request.method == "POST"
                && path == "/batch"
                && request.drop_response
                && value.get("dryRun").is_none()
            {
                // No HTTP status or body is written. Commit completed before the FIN.
                return socket.shutdown().await;
            }
            send(&mut socket, 200, value).await
        }
        Err(error) => send_error(&mut socket, error).await,
    }
}

async fn route(app: &App, request: &Request) -> Result<Value, DomainError> {
    let (path, query) = request.path.split_once('?').unwrap_or((&request.path, ""));
    if let Some(result) = durable_http::route(&app.url, &request.method, path, &request.body).await
    {
        return result;
    }
    match (request.method.as_str(), path) {
        ("GET", "/health") => {
            tokio::time::timeout(std::time::Duration::from_secs(2), async {
                let client = db::connect(&app.url).await?;
                client
                    .simple_query("SELECT 1")
                    .await
                    .map_err(db::internal)?;
                Ok::<(), DomainError>(())
            })
            .await
            .map_err(|_| DomainError::new("INTERNAL_ERROR", "Database health timeout"))??;
            Ok(json!({"ok":true,"engine":"postgres","cacheMetrics":cache_metrics::snapshot()}))
        }
        ("POST", "/projects") => compatibility::create(&app.url, &request.body).await,
        ("POST", "/batch") => {
            let batch: BatchRequest = serde_json::from_slice(&request.body).map_err(invalid)?;
            batch::execute(&app.url, &batch).await
        }
        ("GET", path) if path.starts_with("/projects/") && path[10..].ends_with("/revision") => {
            let id = &path[10..path.len() - 9];
            if id.is_empty()
                || id.len() > 128
                || !id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
            {
                return Err(invalid("Invalid project ID"));
            }
            let revision = db::current_revision(&app.url, id).await?;
            Ok(json!({"projectId":id,"revision":revision}))
        }
        ("GET", path) if path.starts_with("/projects/") => {
            compatibility::snapshot(&app.url, &path[10..]).await
        }
        ("GET", path) if path.starts_with("/changes/") => {
            events::changes(&app.url, &path[9..], since(query)?).await
        }
        _ => Err(DomainError::new("NOT_FOUND", "Unknown route")),
    }
}

fn since(query: &str) -> Result<u64, DomainError> {
    if query.is_empty() {
        return Ok(0);
    }
    query
        .strip_prefix("sinceRevision=")
        .ok_or_else(|| invalid("Expected sinceRevision query"))?
        .parse()
        .map_err(invalid)
}

fn invalid(error: impl std::fmt::Display) -> DomainError {
    DomainError::new("VALIDATION_FAILED", error.to_string())
}
