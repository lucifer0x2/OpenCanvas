use opencanvas_server::{router, AppState, Config};
use opencanvas_storage::{PostgresBackend, SnapshotCache, Storage};
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

pub struct Server {
    pub url: String,
    pub client: Client,
    session: String,
    sequence: AtomicU64,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Server {
    pub async fn start() -> Self {
        Self::start_with_inbox(None).await
    }

    /// `inbox_root` is the local directory holding `<slug>/inbox`, as `--data-root` sets it.
    pub async fn start_with_inbox(inbox_root: Option<PathBuf>) -> Self {
        let dsn = std::env::var("TASK19_POSTGRES_URL")
            .expect("run scripts/task19-check.sh: fresh real PostgreSQL required");
        let storage = tokio::task::spawn_blocking(move || {
            let backend = PostgresBackend::connect(&dsn).unwrap();
            let cache = SnapshotCache::new(backend.cache_namespace());
            let storage = Storage::new(backend.with_cache(cache).unwrap());
            match inbox_root {
                Some(root) => storage.with_inbox_root(root),
                None => storage,
            }
        })
        .await
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = Config::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            listener.local_addr().unwrap().port(),
        );
        let url = config.base_url.clone();
        let state = AppState::new(storage, config);
        let task = tokio::spawn(async move {
            axum::serve(listener, router(state)).await.unwrap();
        });
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        let response = client.post(format!("{url}/mcp"))
            .header("accept", "application/json, text/event-stream")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"real-postgres-contract","version":"1"}}}))
            .send().await.unwrap();
        assert!(response.status().is_success());
        let session = response.headers()["mcp-session-id"]
            .to_str()
            .unwrap()
            .into();
        let _ = response.bytes().await.unwrap();
        let server = Self {
            url,
            client,
            session,
            sequence: AtomicU64::new(2),
            task,
        };
        assert!(server
            .request(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await
            .status()
            .is_success());
        server
    }

    async fn request(&self, body: Value) -> reqwest::Response {
        self.client
            .post(format!("{}/mcp", self.url))
            .header("accept", "application/json, text/event-stream")
            .header("mcp-session-id", &self.session)
            .header("mcp-protocol-version", "2025-11-25")
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    pub async fn call(&self, name: &str, arguments: Value) -> Value {
        let response = self.request(json!({"jsonrpc":"2.0","id":self.sequence.fetch_add(1,Ordering::Relaxed),"method":"tools/call","params":{"name":name,"arguments":arguments}})).await;
        let text = response.text().await.unwrap();
        let raw = text
            .lines()
            .filter_map(|l| l.strip_prefix("data: "))
            .find(|l| !l.is_empty())
            .unwrap_or(&text);
        let value: Value =
            serde_json::from_str(raw).unwrap_or_else(|_| panic!("Bad MCP response: {text}"));
        assert!(value.get("error").is_none(), "{value}");
        value["result"]["structuredContent"].clone()
    }

    pub async fn get(&self, path: &str) -> Value {
        let response = self
            .client
            .get(format!("{}{path}", self.url))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success(), "{}", response.status());
        response.json().await.unwrap()
    }
}
