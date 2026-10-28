//! `render_frame` responses stay small by default: the computed scene is opt-in with
//! `includeScene:true`, and the HTTP job route still returns the full record.
#[allow(dead_code)] // `design` is unused here.
mod support;
use serde_json::{json, Value};
use support::Server;

const NODES: usize = 300;
/// The whole `tools/call` result (text content plus structuredContent) must fit this.
const DEFAULT_BUDGET: usize = 4096;

/// A large frame (300 text and rectangle children), so the full scene is far over budget.
async fn large_frame(s: &Server) -> String {
    let p = s.project().await;
    let mut ops = vec![
        json!({"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"name":"Home"}}),
        json!({"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":400,"height":3700,"fill":"#ffffff"}}),
    ];
    for i in 0..NODES {
        ops.push(if i % 2 == 0 {
            json!({"op":"create_node","id":format!("n{i:03}"),"kind":"rectangle","parentId":"frame","props":{"x":0,"y":i*12,"width":40,"height":10,"fill":"#336699"}})
        } else {
            json!({"op":"create_node","id":format!("n{i:03}"),"kind":"text","parentId":"frame","props":{"x":50,"y":i*12,"text":format!("Dòng chữ số {i}"),"width":300,"height":"hug","fontId":"font_bundled","fontSize":10,"lineHeight":12}})
        });
    }
    let done = s.batch(&p, 0, "many", Value::Array(ops)).await;
    assert_eq!(done["committedRevision"], 1, "{done}");
    p
}
/// The raw `tools/call` result and its serialized size.
async fn render(s: &Server, args: Value) -> (Value, usize) {
    let result = s
        .rpc(
            "tools/call",
            json!({"name":"render_frame","arguments":args}),
        )
        .await;
    assert_ne!(result["isError"], true, "{result}");
    let size = serde_json::to_vec(&result).unwrap().len();
    (result["structuredContent"].clone(), size)
}
fn without_scene(mut job: Value) -> Value {
    job.as_object_mut().unwrap().remove("scene");
    job
}

#[tokio::test]
async fn default_response_is_small_and_scene_is_opt_in() {
    let s = Server::start().await;
    let p = large_frame(&s).await;
    let enqueue = json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":400,"height":3700}});
    let (pending, size) = render(&s, enqueue.clone()).await;
    assert!(size < DEFAULT_BUDGET, "enqueue response is {size} bytes");
    let job_id = pending["jobId"].as_str().unwrap().to_owned();

    let poll = json!({"projectId":p,"jobId":job_id,"waitMs":10_000});
    let (mut compact, mut size) = render(&s, poll.clone()).await;
    for _ in 0..20 {
        if compact["status"] != "pending" {
            break;
        }
        (compact, size) = render(&s, poll.clone()).await;
    }
    assert_eq!(compact["status"], "ready", "{compact}");
    assert!(
        size < DEFAULT_BUDGET,
        "default poll response is {size} bytes"
    );
    assert!(compact.get("scene").is_none(), "{compact}");
    for field in [
        "jobId",
        "sceneKey",
        "imageUrl",
        "image",
        "provenance",
        "readiness",
    ] {
        assert!(compact.get(field).is_some(), "missing {field}: {compact}");
    }
    assert!(compact["diagnostics"].is_array(), "{compact}");

    // The HTTP job route (the viewer's path) still carries the full scene, far over budget.
    let full = s.get(&format!("/api/v1/jobs/{job_id}")).await;
    assert_eq!(full["scene"]["nodes"].as_array().unwrap().len(), NODES + 1);
    let full_size = serde_json::to_vec(&full).unwrap().len();
    assert!(
        full_size > 8 * DEFAULT_BUDGET,
        "full job is {full_size} bytes"
    );
    eprintln!("render_frame default result={size} bytes; full job record={full_size} bytes");
    assert_eq!(compact, without_scene(full.clone()));

    // Opt-in returns the record exactly as before, on a poll and on a cached enqueue.
    let mut opt_in = poll.clone();
    opt_in["includeScene"] = json!(true);
    assert_eq!(render(&s, opt_in).await.0, full);
    let mut cached = enqueue.clone();
    cached["includeScene"] = json!(true);
    assert_eq!(render(&s, cached).await.0, full);
    assert_eq!(render(&s, enqueue).await.0, compact);

    // Without the inline scene, bounds and lines stay readable by scene key.
    let inspect = s
        .call(
            "inspect_layout",
            json!({"projectId":p,"jobId":job_id,"sceneKey":compact["sceneKey"],"nodeIds":["n001"]}),
        )
        .await;
    assert_eq!(inspect["nodes"][0]["id"], "n001", "{inspect}");
    assert!(!inspect["nodes"][0]["lines"].as_array().unwrap().is_empty());
}
#[tokio::test]
async fn include_scene_is_published_and_validated() {
    let s = Server::start().await;
    let tools = s.rpc("tools/list", json!({})).await;
    let render = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "render_frame")
        .unwrap();
    let schema = &render["inputSchema"];
    assert_eq!(schema["properties"]["includeScene"]["type"], "boolean");
    assert_eq!(schema["required"], json!(["projectId"]));
    let p = s.project().await;
    let bad = s
        .call(
            "render_frame",
            json!({"projectId":p,"jobId":"job_missing","includeScene":"yes"}),
        )
        .await;
    assert_eq!(bad["error"]["code"], "INVALID_REQUEST", "{bad}");
    let missing = s
        .call(
            "render_frame",
            json!({"projectId":p,"jobId":"job_missing","includeScene":true}),
        )
        .await;
    assert_eq!(missing["error"]["code"], "JOB_NOT_FOUND", "{missing}");
}
