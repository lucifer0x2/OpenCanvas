#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

fn warnings(value: &Value) -> Vec<&Value> {
    value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"] == "TEXT_CONTENT_OUTSIDE_BOX")
        .collect()
}

#[tokio::test]
async fn measured_overflow_render_inspect_context_scope_and_paging() {
    let s = Server::start().await;
    let p = s.project().await;
    let operations = json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":320,"height":200,"clip":true}},
        {"op":"create_node","id":"overflow","kind":"text","parentId":"frame","props":{"text":"Private paragraph copy","textWrap":"nowrap","x":300,"width":20,"height":8,"fontId":"font_bundled","fontSize":20,"lineHeight":28}},
        {"op":"create_node","id":"clean","kind":"text","parentId":"frame","props":{"text":"OK","x":40,"y":60,"width":200,"height":40,"fontId":"font_bundled","fontSize":20,"lineHeight":28}},
        {"op":"create_note","id":"intent","nodeId":"overflow","text":"Intentional overflow: display lettering escapes the box"}
    ]);
    let result = s.batch(&p, 0, "overflow", operations).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    let job = s.call("render_frame",json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":320,"height":200}})).await;
    assert!(job["jobId"].is_string(), "{job}");
    let job = s.wait_job(job["jobId"].as_str().unwrap()).await;
    assert_eq!(job["status"], "ready", "{job}");
    assert!(job.get("error").is_none(), "{job}");
    let warning = warnings(&job)[0];
    assert_eq!(warnings(&job).len(), 1);
    assert_eq!(warning["nodeId"], "overflow");
    assert_eq!(warning["intentionalOverflow"], true);
    assert_eq!(warning["severity"], "info");
    assert_eq!(warning["inkEvidence"], "measured");
    assert_eq!(warning["examples"][0]["clipNodeIds"], json!(["frame"]));
    assert_eq!(warning["sourceSceneKey"], job["sceneKey"]);
    assert!(warning.get("provenance").is_none());
    // The full provenance travels once per job, on its summary.
    let summary = job["diagnostics"].as_array().unwrap().iter();
    let summary: Vec<_> = summary
        .filter(|d| d["code"] == "TEXT_OVERFLOW_SUMMARY")
        .collect();
    assert_eq!(summary.len(), 1);
    assert_eq!(summary[0]["sourceSceneKey"], job["sceneKey"]);
    assert_eq!(summary[0]["provenance"]["sourceSceneKey"], job["sceneKey"]);
    assert_eq!(
        summary[0]["provenance"]["fontManifestHash"],
        job["provenance"]["fontManifestHash"]
    );
    assert_eq!(
        (
            summary[0]["reportedNodes"].as_u64(),
            summary[0]["omittedNodes"].as_u64()
        ),
        (Some(1), Some(0))
    );
    assert!(!warning.to_string().contains("Private paragraph"));
    let args = json!({"projectId":p,"jobId":job["jobId"],"sceneKey":job["sceneKey"],"fields":["bounds"],"limit":1});
    let first = s.call("inspect_layout", args.clone()).await;
    let mut next = args.clone();
    next["cursor"] = first["nextCursor"].clone();
    let next = s.call("inspect_layout", next).await;
    assert_eq!(next["diagnostics"], first["diagnostics"]);
    let mut filtered = args;
    filtered["nodeIds"] = json!(["clean"]);
    let clean = s.call("inspect_layout", filtered).await;
    assert!(warnings(&clean).is_empty());
    let args = json!({"projectId":p,"frameId":"frame","viewports":[{"width":320,"height":200}],"properties":["name"],"limit":1});
    let context = s.call("get_design_context", args.clone()).await;
    for vp in context["computed"]["viewports"].as_array().unwrap() {
        s.wait_job(vp["jobId"].as_str().unwrap()).await;
    }
    let context = s.call("get_design_context", args.clone()).await;
    assert_eq!(warnings(&context), warnings(&job));
    let mut next = args;
    next["cursor"] = context["nextCursor"].clone();
    let page = s.call("get_design_context", next).await;
    assert_eq!(page["diagnostics"], context["diagnostics"]);
    let change = s
        .batch(
            &p,
            1,
            "later",
            json!([{"op":"update_node","id":"overflow","props":{"width":400,"height":100,"x":0}}]),
        )
        .await;
    assert_eq!(change["committedRevision"], 2, "{change}");
    let old = s.call("inspect_layout",json!({"projectId":p,"jobId":job["jobId"],"sceneKey":job["sceneKey"],"nodeIds":["overflow"],"fields":["bounds"]})).await;
    assert_eq!(warnings(&old), warnings(&job));
    println!("overflow evidence: {}", warning);
}
