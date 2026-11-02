mod batch;
mod batch_write;
mod blobs;
mod cache_metrics;
mod compatibility;
mod db;
mod delta;
mod durable_http;
mod events;
mod hot_reads;
mod hot_snapshot;
mod http;
mod jobs;
mod leases;
mod profile;
mod redis_cache;
mod staging;
mod transport;

use std::sync::Arc;
use tokio::net::TcpListener;

pub struct App {
    url: String,
    cache: Option<redis_cache::RedisCache>,
    wake: tokio::sync::Notify,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("PG_URL")?;
    let client = db::connect(&url).await.map_err(|e| e.message)?;
    db::initialize(&*client).await.map_err(|e| e.message)?;
    drop(client);
    let port = std::env::var("PORT").unwrap_or_else(|_| "0".into());
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    let cache = redis_cache::RedisCache::from_env(&url);
    let app = Arc::new(App {
        url,
        cache,
        wake: tokio::sync::Notify::new(),
    });
    tokio::spawn(events::listen(app.clone()));
    println!(
        "{}",
        serde_json::json!({"ready":true,"port":listener.local_addr()?.port(),"engine":"postgres","compatVersion":if cfg!(feature="compat-v2") {2} else {1}})
    );
    let mut requests = tokio::task::JoinSet::new();
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            Some(_) = requests.join_next(), if !requests.is_empty() => {},
            incoming = listener.accept() => {
                let (stream, _) = incoming?;
                let app = app.clone();
                requests.spawn(async move {
                    if let Err(error) = http::serve(stream, app).await {
                        eprintln!("http: {error}");
                    }
                });
            }
        }
    }
    drop(listener);
    // Drain finite requests; SSE peers reconnect to another gateway. Jobs are in DB.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while requests.join_next().await.is_some() {}
    })
    .await;
    requests.abort_all();
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("register SIGTERM");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
