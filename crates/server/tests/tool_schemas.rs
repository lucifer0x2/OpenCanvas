//! Published tool input schemas must be plain top-level objects: MCP requires `"type":"object"`
//! and the Anthropic API rejects top-level combinators, so one bad schema fails the whole
//! `tools/list` for those clients. `render_frame` still enforces its two exclusive modes at
//! runtime. No browser is needed: accepted calls stop at a later, non-schema error.
#[allow(dead_code)] // `root`, `state`, `batch`, `get`, `wait_job` and `design` are unused here.
mod support;
mod tool_names;
use serde_json::{json, Value};
use support::Server;

const INVALID: &str =
    "Arguments do not match the published tool schema; check required fields, types and limits";

#[tokio::test]
async fn every_published_input_schema_is_a_plain_object() {
    let s = Server::start().await;
    let tools = s.rpc("tools/list", json!({})).await;
    let tools = tools["tools"].as_array().unwrap();
    tool_names::assert_listed(tools);
    for tool in tools {
        let schema = tool["inputSchema"].as_object().unwrap();
        let name = &tool["name"];
        assert_eq!(schema.get("type"), Some(&json!("object")), "{name}");
        assert!(
            schema.get("properties").is_some_and(Value::is_object),
            "{name}"
        );
        for combinator in ["oneOf", "anyOf", "allOf", "not"] {
            assert!(!schema.contains_key(combinator), "{name}: {combinator}");
        }
    }
    let render = tools.iter().find(|t| t["name"] == "render_frame").unwrap();
    let schema = &render["inputSchema"];
    let mut fields: Vec<_> = schema["properties"].as_object().unwrap().keys().collect();
    fields.sort();
    let expected = [
        "frameId",
        "includeScene",
        "jobId",
        "projectId",
        "revision",
        "scale",
        "state",
        "theme",
        "viewport",
        "waitMs",
    ];
    assert_eq!(fields, expected);
    assert_eq!(schema["required"], json!(["projectId"]));
    assert_eq!(schema["additionalProperties"], false);
}

async fn code(s: &Server, args: Value) -> Value {
    let result = s.call("render_frame", args.clone()).await;
    if result["error"]["code"] == "INVALID_REQUEST" {
        assert_eq!(result["error"]["message"], INVALID, "{args}: {result}");
    }
    result["error"]["code"].clone()
}

#[tokio::test]
async fn render_frame_modes_are_still_exclusive_at_runtime() {
    let s = Server::start().await;
    let p = s.project().await;
    let viewport = json!({"width":320,"height":200});
    // Accepted shapes pass the schema and stop at the job or revision lookup.
    let enqueue = json!({"projectId":p,"revision":7,"frameId":"frame","viewport":viewport});
    assert_eq!(code(&s, enqueue.clone()).await, "REVISION_UNAVAILABLE");
    let full = json!({"projectId":p,"revision":7,"frameId":"frame","viewport":viewport,"theme":"night","state":"default","scale":2});
    assert_eq!(code(&s, full).await, "REVISION_UNAVAILABLE");
    let poll = json!({"projectId":p,"jobId":"job_missing"});
    assert_eq!(code(&s, poll.clone()).await, "JOB_NOT_FOUND");
    let wait = json!({"projectId":p,"jobId":"job_missing","waitMs":100});
    assert_eq!(code(&s, wait).await, "JOB_NOT_FOUND");
    // Mixed modes, waitMs without jobId, neither mode, unknown fields and bad values.
    let mut rejected = vec![
        json!({"projectId":p,"jobId":"job_missing","frameId":"frame"}),
        json!({"projectId":p,"jobId":"job_missing","revision":7}),
        json!({"projectId":p,"jobId":"job_missing","viewport":viewport}),
        json!({"projectId":p,"jobId":"job_missing","theme":"night"}),
        json!({"projectId":p,"waitMs":100}),
        json!({"projectId":p}),
        json!({}),
        json!({"jobId":"job_missing"}),
        json!({"projectId":p,"jobId":"job_missing","extra":1}),
        json!({"projectId":p,"jobId":"job_missing","waitMs":10_001}),
        json!({"projectId":p,"revision":7,"frameId":"frame"}),
    ];
    for extra in [
        json!({"jobId":"job_missing"}),
        json!({"waitMs":100}),
        json!({"extra":1}),
        json!({"scale":5}),
        json!({"state":"hover"}),
    ] {
        let mut args = enqueue.clone();
        args.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        rejected.push(args);
    }
    for args in rejected {
        assert_eq!(code(&s, args.clone()).await, "INVALID_REQUEST", "{args}");
    }
}
