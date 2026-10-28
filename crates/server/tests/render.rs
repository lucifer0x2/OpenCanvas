mod support;
use serde_json::{json, Value};
use support::{design, Server};

async fn render(s: &Server, p: &str, revision: u64, width: u64) -> Value {
    s.call("render_frame",json!({"projectId":p,"revision":revision,"frameId":"frame","viewport":{"width":width,"height":200}})).await
}
#[tokio::test]
async fn real_chromium_png_exact_revision_and_theme_cache() {
    let s = Server::start().await;
    let p = s.project().await;
    assert_eq!(
        s.batch(&p, 0, "initial", design()).await["committedRevision"],
        1
    );
    let pending = render(&s, &p, 1, 320).await;
    let old_id = pending["jobId"].as_str().unwrap();
    let next=s.batch(&p,1,"newer",json!([
        {"op":"upsert_token","token":{"id":"fill","name":"Fill","type":"color","value":{"default":"#ffffff","themes":{"night":"#101010"}}}},
        {"op":"update_node","id":"frame","props":{"fill":{"tokenId":"fill"}}}
    ])).await;
    assert_eq!(next["committedRevision"], 2, "{next}");
    let old = s.wait_job(old_id).await;
    assert_eq!(old["status"], "ready", "{old}");
    assert!(old.get("error").is_none(), "{old}");
    assert_eq!(old["revision"], 1);
    assert_eq!(old["scene"]["provenance"]["revision"], 1);
    assert_eq!(
        old["sceneKey"],
        old["scene"]["provenance"]["sourceSceneKey"]
    );
    assert!(old["provenance"]["browserVersion"].is_string());
    assert!(old["provenance"]["runtimeVersion"]
        .as_str()
        .unwrap()
        .starts_with('v'));
    assert_eq!(old["readiness"][0]["status"], "ready");
    let png = s
        .client
        .get(old["imageUrl"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(png.headers()["content-type"], "image/png");
    let bytes = png.bytes().await.unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 320);
    assert_eq!(
        render(&s, &p, 1, 320).await["error"]["code"],
        "REVISION_UNAVAILABLE"
    );
    let light = render(&s, &p, 2, 320).await;
    let dark=s.call("render_frame",json!({"projectId":p,"revision":2,"frameId":"frame","viewport":{"width":320,"height":200},"theme":"night"})).await;
    assert_ne!(light["sceneKey"], dark["sceneKey"]);
    let dark = s.wait_job(dark["jobId"].as_str().unwrap()).await;
    assert_eq!(dark["scene"]["nodes"][0]["fill"], "#101010");
    let inspect = s
        .call(
            "inspect_layout",
            json!({"projectId":p,"jobId":old_id,"sceneKey":old["sceneKey"],"nodeIds":["text"]}),
        )
        .await;
    assert_eq!(inspect["revision"], 1);
    assert_eq!(inspect["nodes"][0]["id"], "text");
}

#[tokio::test]
async fn missing_image_has_diagnosable_scene_and_strict_failure_then_recovers() {
    let s = Server::start().await;
    let p = s.project().await;
    let imported=s.call("import_asset",json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"image","mediaType":"image/png","base64":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII="})).await;
    assert!(imported.get("error").is_none(), "{imported}");
    let snapshot = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    let asset = snapshot["assets"][0]["id"].clone();
    let mut ops = design();
    ops.as_array_mut().unwrap().push(json!({"op":"create_node","id":"image","kind":"image","parentId":"frame","props":{"assetId":asset,"width":32,"height":32}}));
    let files: Vec<_> = std::fs::read_dir(s.root.path().join("test-project/assets"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .collect();
    assert_eq!(files.len(), 1);
    let binary = std::fs::read(&files[0]).unwrap();
    std::fs::remove_file(&files[0]).unwrap();
    assert_eq!(s.batch(&p, 1, "design", ops).await["committedRevision"], 2);
    let pending = render(&s, &p, 2, 319).await;
    let job = s.wait_job(pending["jobId"].as_str().unwrap()).await;
    assert_eq!(job["status"], "ready", "{job}");
    assert!(job["error"].is_object(), "{job}");
    assert!(job["diagnostics"].as_array().unwrap().len() > 0);
    assert!(job.get("imageUrl").is_none());
    assert_eq!(
        s.get(&format!(
            "/api/v1/jobs/{}/image",
            job["jobId"].as_str().unwrap()
        ))
        .await["error"]["code"],
        "IMAGE_NOT_READY"
    );
    std::fs::write(&files[0], binary).unwrap();
    let retry = render(&s, &p, 2, 319).await;
    assert_ne!(retry["jobId"], job["jobId"]);
    let retry = s.wait_job(retry["jobId"].as_str().unwrap()).await;
    assert!(retry.get("imageUrl").is_some(), "{retry}");
}
