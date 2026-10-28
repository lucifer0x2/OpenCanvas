mod support;
use serde_json::{json, Value};
use support::{design, Server};

async fn settled(s: &Server, args: Value) -> Value {
    let result = s.call("get_design_context", args.clone()).await;
    for item in result["computed"]["viewports"].as_array().unwrap() {
        s.wait_job(item["jobId"].as_str().unwrap()).await;
    }
    s.call("get_design_context", args).await
}

#[tokio::test]
async fn missing_resource_cannot_claim_comparison_success_and_explicit_retry_recovers() {
    let s = Server::start().await;
    let p = s.project().await;
    let import = s.call("import_asset", json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"image","mediaType":"image/png","base64":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII="})).await;
    assert!(import.get("error").is_none(), "{import}");
    let snapshot = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    let asset = snapshot["assets"][0]["id"].clone();
    let path = std::fs::read_dir(s.root.path().join("test-project/assets"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_file())
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let mut ops = design();
    ops.as_array_mut().unwrap().push(json!({"op":"create_node","id":"image","kind":"image","parentId":"frame","props":{"assetId":asset,"width":20,"height":20}}));
    assert_eq!(s.batch(&p, 1, "design", ops).await["committedRevision"], 2);
    let args = json!({"projectId":p,"frameId":"frame","viewports":[{"width":319,"height":200},{"width":239,"height":200}]});
    let result = settled(&s, args.clone()).await;
    assert_eq!(result["computed"]["status"], "failed", "{result}");
    assert!(result["reflow"].is_null());
    for item in result["computed"]["viewports"].as_array().unwrap() {
        assert_eq!(item["status"], "failed");
        assert!(item["error"]["code"].is_string());
    }
    assert_eq!(result["nodes"][1]["computedBounds"], json!([null, null]));
    std::fs::write(&path, bytes).unwrap();
    for item in result["computed"]["viewports"].as_array().unwrap() {
        let retry = s
            .call(
                "render_frame",
                json!({"projectId":p,"revision":2,"frameId":"frame","viewport":item["viewport"]}),
            )
            .await;
        assert_ne!(retry["jobId"], item["jobId"]);
        let job = s.wait_job(retry["jobId"].as_str().unwrap()).await;
        assert_eq!(job["status"], "ready", "{job}");
        assert!(job.get("error").is_none(), "{job}");
    }
    let ready = s.call("get_design_context", args).await;
    assert_eq!(ready["computed"]["status"], "ready");
    assert_eq!(ready["reflow"], false);
    assert_eq!(
        ready["nodes"][1]["computedBounds"][0],
        ready["nodes"][1]["computedBounds"][1]
    );
}

#[tokio::test]
async fn cold_text_measurement_is_async_and_uses_the_same_scene_cache_as_render() {
    let s = Server::start().await;
    let p = s.project().await;
    let mut ops = design();
    ops[1]["props"]["layout"] = json!({"mode":"column"});
    ops[2]["props"]["width"] = json!("fill");
    let req = serde_json::from_value(
        json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"cold","operations":ops}),
    )
    .unwrap();
    let id = p.clone();
    s.state
        .db(move |store| store.open_project(&id)?.apply_batch(req))
        .await
        .unwrap();
    let args = json!({"projectId":p,"frameId":"frame","viewports":[{"width":320,"height":200},{"width":160,"height":200}]});
    let start = std::time::Instant::now();
    let pending = s.call("get_design_context", args.clone()).await;
    let read_time = start.elapsed();
    assert_eq!(pending["computed"]["status"], "pending");
    let ready = settled(&s, args).await;
    eprintln!(
        "cold worker (no automatic render): enqueue_read={read_time:?} total_to_ready={:?}",
        start.elapsed()
    );
    assert_eq!(ready["computed"]["status"], "ready", "{ready}");
    assert_eq!(ready["reflow"], true);
    assert_ne!(
        ready["nodes"][1]["computedBounds"][0],
        ready["nodes"][1]["computedBounds"][1]
    );
    for item in ready["computed"]["viewports"].as_array().unwrap() {
        let render = s
            .call(
                "render_frame",
                json!({"projectId":p,"frameId":"frame","revision":1,"viewport":item["viewport"],"includeScene":true}),
            )
            .await;
        assert_eq!(render["jobId"], item["jobId"]);
        assert_eq!(render["sceneKey"], item["sceneKey"]);
        assert_eq!(render["status"], "ready");
        assert!(render["scene"]["nodes"][1]["lines"]
            .as_array()
            .is_some_and(|lines| !lines.is_empty()));
    }
}
