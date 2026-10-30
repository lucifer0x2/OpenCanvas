//! Actual compact JSON result bytes (excluding JSON-RPC/SSE) of a repeatable workflow.
#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

fn bytes(value: &Value) -> usize {
    serde_json::to_vec(value).unwrap().len()
}

#[tokio::test]
async fn measure_discovery_workflow() {
    let s = Server::start().await;
    let p = s.project().await;
    let result = s.batch(&p, 0, "fixture", support::design()).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    let mut total = 0;
    for (step, method, args) in [
        ("tools", "tools/list", json!({})),
        ("resources", "resources/list", json!({})),
        (
            "author",
            "resources/read",
            json!({"uri":"skill://opencanvas/author-frame.md"}),
        ),
        (
            "implement",
            "resources/read",
            json!({"uri":"skill://opencanvas/implement-frame.md"}),
        ),
        (
            "props",
            "resources/read",
            json!({"uri":"skill://opencanvas/props.md"}),
        ),
    ] {
        let result = s.rpc(method, args).await;
        let size = bytes(&result);
        if step == "tools" {
            println!(
                "discovery-workflow: tools-array={} B (result wrapper excluded)",
                bytes(&result["tools"])
            );
        }
        total += size;
        println!("discovery-workflow: {step}={size} B");
    }
    let listed = s.rpc("resources/list", json!({})).await;
    if listed["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["uri"] == "opencanvas://capabilities")
    {
        let value = s
            .rpc("resources/read", json!({"uri":"opencanvas://capabilities"}))
            .await;
        let size = bytes(&value);
        total += size;
        println!("discovery-workflow: capabilities={size} B (once; cacheable)");
    }
    let context = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"frame","compact":true}),
        )
        .await;
    assert_eq!(context["nodes"].as_array().unwrap().len(), 2);
    let size = bytes(&context);
    total += size;
    println!("discovery-workflow: context={size} B; total={total} B");
    assert!(total < 100_000);
}
