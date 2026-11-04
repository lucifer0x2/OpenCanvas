//! One protocol client shared by in-process fixtures and the actual empty-PATH binary test.
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

pub struct Mcp {
    pub url: String,
    pub client: Client,
    session: String,
    sequence: AtomicU64,
}
impl Mcp {
    pub async fn connect(url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap();
        let response = client.post(format!("{url}/mcp"))
            .header("accept", "application/json, text/event-stream")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"native-test","version":"1"}}}))
            .send().await.unwrap();
        let session = response.headers()["mcp-session-id"]
            .to_str()
            .unwrap()
            .into();
        let server = Self {
            url,
            client,
            session,
            sequence: AtomicU64::new(2),
        };
        server
            .request(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await;
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
        let response = self.request(json!({"jsonrpc":"2.0","id":self.sequence.fetch_add(1, Ordering::Relaxed),"method":"tools/call","params":{"name":name,"arguments":arguments}})).await;
        let text = response.text().await.unwrap();
        let raw = text
            .lines()
            .filter_map(|l| l.strip_prefix("data: "))
            .find(|l| !l.is_empty())
            .unwrap_or(&text);
        let result: Value = serde_json::from_str(raw).unwrap();
        assert!(result.get("error").is_none(), "{result}");
        result["result"]["structuredContent"].clone()
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
    pub async fn project(&self) -> String {
        self.call(
            "create_project",
            json!({"slug":"native","name":"Native test"}),
        )
        .await["projectId"]
            .as_str()
            .unwrap()
            .into()
    }
    pub async fn batch(&self, project: &str, revision: u64, operations: Value) -> Value {
        self.call("apply_batch", json!({"projectId":project,"expectedRevision":revision,"idempotencyKey":format!("batch-{revision}"),"operations":operations})).await
    }
    pub async fn render(&self, project: &str, revision: u64, state: &str) -> Value {
        let pending = self.call("render_frame", json!({"projectId":project,"revision":revision,"frameId":"frame","viewport":{"width":320,"height":200},"state":state})).await;
        assert!(pending["jobId"].is_string(), "{pending}");
        self.wait(pending["jobId"].as_str().unwrap()).await
    }
    pub async fn wait(&self, id: &str) -> Value {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let job = self.get(&format!("/api/v1/jobs/{id}")).await;
                if job["status"] != "pending" {
                    return job;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("Native job did not settle")
    }
}
