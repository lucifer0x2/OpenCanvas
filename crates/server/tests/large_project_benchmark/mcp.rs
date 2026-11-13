//! Item 5 MCP reads through a real listener: one session, end-to-end tool calls.
use super::snapshots::serve;
use super::source::Targets;
use super::stats::{ms, summary, Plan};
use super::Backends;
use serde_json::{json, Value};
use std::time::Instant;

struct Session {
    client: reqwest::Client,
    url: String,
    id: String,
}

impl Session {
    async fn open(base: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .unwrap();
        let url = format!("{base}/mcp");
        let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"task24-bench","version":"1"}}});
        let response = client
            .post(&url)
            .header("accept", "application/json, text/event-stream")
            .json(&init)
            .send()
            .await
            .unwrap();
        let id = response.headers()["mcp-session-id"]
            .to_str()
            .unwrap()
            .to_owned();
        response.text().await.unwrap();
        let session = Self { client, url, id };
        session
            .post(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await;
        session
    }

    async fn post(&self, body: Value) -> String {
        let response = self
            .client
            .post(&self.url)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-session-id", &self.id)
            .header("mcp-protocol-version", "2025-11-25")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success(), "{}", response.status());
        response.text().await.unwrap()
    }

    /// The structured result and the raw response size, or the tool's error object.
    async fn call(&self, name: &str, arguments: Value) -> (Result<Value, Value>, usize) {
        let body = json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":name,"arguments":arguments}});
        let text = self.post(body).await;
        let raw = text
            .lines()
            .filter_map(|l| l.strip_prefix("data: "))
            .find(|l| !l.is_empty())
            .unwrap_or(&text);
        let value: Value = serde_json::from_str(raw).unwrap();
        let result = &value["result"];
        let outcome = if let Some(error) = value.get("error") {
            Err(error.clone())
        } else if result["isError"] == true {
            Err(result["structuredContent"]["error"].clone())
        } else {
            Ok(result["structuredContent"].clone())
        };
        if let Err(error) = &outcome {
            eprintln!(
                "Task24 {name} error: {} {}",
                error["code"], error["message"]
            );
        }
        (outcome, text.len())
    }
}

pub async fn reads(plan: &Plan, backends: &Backends, project: &str, targets: &Targets) -> Value {
    let (base, task) = serve(backends).await;
    let session = Session::open(&base).await;
    let calls = [
        (
            "findNameManyMatches",
            "find_nodes",
            json!({"projectId":project,"nameContains":"a"}),
        ),
        (
            "findNameNoMatch",
            "find_nodes",
            json!({"projectId":project,"nameContains":"zz-task24-absent"}),
        ),
        (
            "findTextContains",
            "find_nodes",
            json!({"projectId":project,"textContains":"e"}),
        ),
        (
            "outlinePage",
            "get_outline",
            json!({"projectId":project,"nodeId":targets.page}),
        ),
        (
            "designContextFrame",
            "get_design_context",
            json!({"projectId":project,"frameId":targets.frame}),
        ),
    ];
    let mut out = json!({});
    for (label, tool, arguments) in calls {
        let mut values = Vec::new();
        let mut last = (Ok(Value::Null), 0);
        for index in 0..plan.total() {
            let start = Instant::now();
            last = session.call(tool, arguments.clone()).await;
            if plan.measured(index) {
                values.push(ms(start));
            }
        }
        let (outcome, bytes) = last;
        // A failing read is still timed; the report flags it by `error`.
        let error = outcome.as_ref().err().map(|e| json!({"code": e["code"]}));
        let result = outcome.unwrap_or_default();
        out[label] = json!({
            "tool": tool, "time": summary(&values), "responseBytes": bytes, "error": error,
            "scannedNodes": result.get("scannedNodes"), "totalMatches": result.get("totalMatches"),
            "truncated": result.get("truncated"),
        });
    }
    task.abort();
    out
}
