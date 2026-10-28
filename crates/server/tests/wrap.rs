mod multi_support;
mod support;
use multi_support::settled;
use serde_json::{json, Value};
use support::Server;

#[tokio::test]
async fn actual_mcp_wrap_bounds_and_atomic_errors() {
    let s = Server::start().await;
    let p = s.project().await;
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "-e", "import {wrapOperations} from './spikes/e2e/wrap-design.mjs';console.log(JSON.stringify(wrapOperations()));"])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ops: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        s.batch(&p, 0, "wrap", ops.clone()).await["committedRevision"],
        1
    );
    assert_eq!(s.batch(&p, 0, "wrap", ops).await["committedRevision"], 1);
    let context = settled(
        &s,
        json!({"projectId":p,"frameId":"frame","viewports":[
            {"width":1040,"height":844},{"width":640,"height":844},{"width":390,"height":844}
        ]}),
    )
    .await;
    assert_eq!(context["reflow"], true, "{context}");
    let nodes = context["nodes"].as_array().unwrap();
    let row = nodes.iter().find(|n| n["id"] == "cards").unwrap();
    assert_eq!(row["props"]["layout"]["wrap"], true);
    assert_eq!(row["resolvedProps"]["layout"]["wrap"], true);
    assert_eq!(
        row["computedBounds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["height"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        vec![128., 224., 320.]
    );
    let before = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    let bad = s
        .batch(
            &p,
            1,
            "bad-wrap",
            json!([
                {"op":"update_node","id":"frame","props":{"name":"rollback"}},
                {"op":"update_node","id":"card-1","props":{"minWidth":0}}
            ]),
        )
        .await;
    assert_eq!(bad["error"]["code"], "VALIDATION_FAILED");
    assert_eq!(
        s.get(&format!("/api/v1/projects/{p}/snapshot")).await,
        before
    );
    let valid = s
        .batch(
            &p,
            1,
            "bad-wrap",
            json!([
                {"op":"update_node","id":"frame","props":{"name":"valid after rejection"}}
            ]),
        )
        .await;
    assert_eq!(valid["committedRevision"], 2);
}
