use opencanvas_server::{router, AppState, Config};
use opencanvas_storage::Store;
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tempfile::TempDir;

pub struct Server {
    pub root: TempDir,
    pub url: String,
    pub client: Client,
    pub session: String,
    pub state: AppState,
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
        Self::configured(|_| {}).await
    }
    pub async fn configured(configure: impl FnOnce(&mut Config)) -> Self {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path()).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut config = Config::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            listener.local_addr().unwrap().port(),
        );
        configure(&mut config);
        let url = config.base_url.clone();
        let state = AppState::new(store, config);
        let app = router(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        let response=client.post(format!("{url}/mcp")).header("accept","application/json, text/event-stream").json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"opencanvas-test","version":"1"}}})).send().await.unwrap();
        assert!(response.status().is_success(), "{}", response.status());
        let session = response.headers()["mcp-session-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let text = response.text().await.unwrap();
        assert!(text.contains("OpenCanvas"), "{text}");
        let server = Self {
            root,
            url,
            client,
            session,
            state,
            sequence: AtomicU64::new(2),
            task,
        };
        let response = server
            .request(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await;
        assert!(response.status().is_success());
        server
    }
    pub async fn request(&self, body: Value) -> reqwest::Response {
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
    pub async fn rpc(&self, method: &str, params: Value) -> Value {
        let response = self
            .request(json!({"jsonrpc":"2.0","id":self.sequence.fetch_add(1,Ordering::Relaxed),"method":method,"params":params}))
            .await;
        let text = response.text().await.unwrap();
        let raw = text
            .lines()
            .filter_map(|l| l.strip_prefix("data: "))
            .find(|l| !l.is_empty())
            .unwrap_or(&text);
        let value: Value =
            serde_json::from_str(raw).unwrap_or_else(|_| panic!("Bad MCP response: {text}"));
        assert!(value.get("error").is_none(), "{value}");
        value["result"].clone()
    }
    pub async fn call(&self, name: &str, args: Value) -> Value {
        let result = self
            .rpc("tools/call", json!({"name":name,"arguments":args}))
            .await;
        result["structuredContent"].clone()
    }
    pub async fn project(&self) -> String {
        self.call(
            "create_project",
            json!({"slug":"test-project","name":"Integration"}),
        )
        .await["projectId"]
            .as_str()
            .unwrap()
            .into()
    }
    pub async fn batch(&self, project: &str, revision: u64, key: &str, operations: Value) -> Value {
        self.call("apply_batch",json!({"projectId":project,"expectedRevision":revision,"idempotencyKey":key,"operations":operations})).await
    }
    pub async fn get(&self, path: &str) -> Value {
        self.client
            .get(format!("{}{path}", self.url))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }
    pub async fn wait_job(&self, id: &str) -> Value {
        for _ in 0..300 {
            let job = self.get(&format!("/api/v1/jobs/{id}")).await;
            if job["status"] != "pending" {
                return job;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("Job did not finish")
    }
}
pub fn design() -> Value {
    json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"name":"Home"}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":320,"height":200,"fill":"#ffffff"}},
    {"op":"create_node","id":"text","kind":"text","parentId":"frame","props":{"text":"Tiếng Việt OpenCanvas","width":280,"height":"hug","fontId":"font_bundled","fontSize":20,"lineHeight":28}}
    ])
}
