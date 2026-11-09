mod native_support;
use base64::{engine::general_purpose::STANDARD, Engine};
use native_support::{design, Server};
use opencanvas_server::{AppState, Config, RenderBackend};
use serde_json::{json, Value};

fn ready_scene(job: &Value) {
    assert_eq!(job["status"], "ready", "{job}");
    assert!(job.get("error").is_none(), "{job}");
    assert_eq!(
        job["scene"]["provenance"]["sourceSceneKey"],
        job["sceneKey"]
    );
    assert_eq!(job["readiness"][0]["status"], "ready");
    assert!(job["imageUrl"].is_string());
    assert_eq!(job["provenance"]["engine"], "native");
    assert!(job["provenance"].get("browserVersion").is_none());
    assert!(!job["scene"]["nodes"][1]["lines"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn native_compute_and_png_without_node_preserve_states_advisories_and_cache() {
    let s = Server::start().await;
    let p = s.project().await;
    let mut events = s.state.events.subscribe();
    assert_eq!(s.batch(&p, 0, design()).await["committedRevision"], 1);
    let default = s.render(&p, 1, "default").await;
    ready_scene(&default);
    let diagnostics = default["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.as_str().is_some_and(|d| d.starts_with("MISSING_GLYPH"))),
        "{default}"
    );
    let overflow = diagnostics
        .iter()
        .find(|d| d["code"] == "TEXT_CONTENT_OUTSIDE_BOX")
        .unwrap();
    assert_eq!(overflow["sourceSceneKey"], default["sceneKey"]);
    assert_eq!(overflow["intentionalOverflow"], true);
    assert!(!overflow.to_string().contains("Private long copy"));
    let hover = s.render(&p, 1, "hover").await;
    ready_scene(&hover);
    assert_eq!(hover["scene"]["nodes"][0]["fill"], "#abcdef");
    assert_eq!(
        hover["scene"]["nodes"][1]["bounds"],
        default["scene"]["nodes"][1]["bounds"]
    );
    assert_eq!(
        hover["scene"]["nodes"][1]["lines"],
        default["scene"]["nodes"][1]["lines"]
    );
    assert_ne!(hover["sceneKey"], default["sceneKey"]);
    let inspect = s.call("inspect_layout", json!({"projectId":p,"jobId":default["jobId"],"sceneKey":default["sceneKey"],"nodeIds":["text"]})).await;
    assert_eq!(inspect["nodes"][0]["id"], "text", "{inspect}");
    let again = s.render(&p, 1, "hover").await;
    assert_eq!(again["jobId"], hover["jobId"]);
    let mut scene_event = false;
    while let Ok(event) = events.try_recv() {
        scene_event |= event["event"] == "scene_ready" && event["sceneKey"] == default["sceneKey"];
    }
    assert!(scene_event);

    // Same input with a distinct actual backend identity cannot share scene keys.
    let mut chromium = Config::for_backend(s.root.path().into(), 0, RenderBackend::Chromium);
    chromium.node = s.root.path().join("no-node").to_string_lossy().into_owned();
    let other = AppState::new(s.state.store.clone(), chromium);
    let request = serde_json::from_value(json!({"frameId":"frame","revision":1,
        "width":320,"height":200}))
    .unwrap();
    let different = other
        .jobs
        .enqueue(s.state.store.snapshot(&p).unwrap(), request)
        .await
        .unwrap();
    assert_ne!(different["sceneKey"], default["sceneKey"]);

    let s = Server::start().await;
    let p = s.project().await;
    let font = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");
    let imported: Value = s.client.post(format!("{}/api/v1/projects/{p}/fonts", s.url))
        .header("origin", &s.url).json(&json!({"expectedRevision":0,"idempotencyKey":"font","mediaType":"font/ttf","base64":STANDARD.encode(font),"font":{"id":"uploaded","family":"Exact Alias","weight":700,"style":"italic","license":"OFL-1.1"}}))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    let mut ops = design();
    ops[2]["props"]["fontId"] = json!("uploaded");
    assert_eq!(s.batch(&p, 1, ops).await["committedRevision"], 2);
    let uploaded = s.render(&p, 2, "hover").await;
    ready_scene(&uploaded);
    assert_eq!(uploaded["scene"]["fonts"][0]["family"], "Exact Alias");
    assert_eq!(uploaded["scene"]["fonts"][0]["weight"], 700);
    assert_eq!(uploaded["scene"]["fonts"][0]["style"], "italic");
    assert_ne!(
        uploaded["scene"]["nodes"][1]["lines"],
        hover["scene"]["nodes"][1]["lines"]
    );
    for path in std::fs::read_dir(s.root.path().join("data/native/assets")).unwrap() {
        let path = path.unwrap().path();
        if path.is_file() {
            std::fs::remove_file(path).unwrap();
        }
    }
    let missing = s.render(&p, 2, "pressed").await;
    assert_eq!(missing["status"], "failed", "{missing}");
    assert_eq!(missing["error"]["code"], "FONT_MISSING");
    assert!(missing.get("scene").is_none());
    assert!(!s.root.path().join("worker").exists());
}

#[tokio::test]
async fn missing_image_preserves_native_scene_and_failed_asset_readiness() {
    let s = Server::start().await;
    let p = s.project().await;
    let imported = s.call("import_asset", json!({"projectId":p,"expectedRevision":0,
        "idempotencyKey":"image","mediaType":"image/png",
        "base64":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII="})).await;
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    let snapshot = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    let asset = snapshot["assets"][0]["id"].clone();
    let mut ops = design();
    ops.as_array_mut()
        .unwrap()
        .push(json!({"op":"create_node","id":"image","kind":"image",
        "parentId":"frame","props":{"assetId":asset,"width":32,"height":32}}));
    for file in std::fs::read_dir(s.root.path().join("data/native/assets")).unwrap() {
        let path = file.unwrap().path();
        if path.is_file() {
            std::fs::remove_file(path).unwrap();
        }
    }
    assert_eq!(s.batch(&p, 1, ops).await["committedRevision"], 2);
    let job = s.render(&p, 2, "default").await;
    assert_eq!(job["status"], "ready", "{job}");
    assert_eq!(job["error"]["code"], "RESOURCE_NOT_READY", "{job}");
    let readiness = job["readiness"].as_array().unwrap();
    assert!(readiness.iter().any(|r| r["id"] == asset
        && r["kind"] == "asset"
        && r["status"] == "failed"
        && r["code"] == "RESOURCE_UNAVAILABLE"));
    assert!(readiness
        .iter()
        .any(|r| r["kind"] == "font" && r["status"] == "ready"));
    assert!(job["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d.as_str().is_some_and(|d| d.contains("failed readiness"))));
    assert_eq!(
        job["scene"]["provenance"]["sourceSceneKey"],
        job["sceneKey"]
    );
    assert!(job.get("imageUrl").is_none());
}

#[tokio::test]
async fn native_scene_with_opaque_diagnostics_produces_downloadable_png() {
    let s = Server::start().await;
    let p = s.project().await;
    assert_eq!(s.batch(&p, 0, design()).await["committedRevision"], 1);
    let job = s.render(&p, 1, "hover").await;
    assert_eq!(job["status"], "ready", "{job}");
    assert!(job.get("error").is_none(), "{job}");
    assert_eq!(job["scene"]["nodes"][0]["fill"], "#abcdef");
    assert_eq!(
        job["scene"]["provenance"]["sourceSceneKey"],
        job["sceneKey"]
    );
    assert!(job["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(Value::is_object));
    assert_eq!(job["readiness"][0]["kind"], "font");
    assert_eq!(job["readiness"][0]["status"], "ready");
    let png = s
        .client
        .get(job["imageUrl"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(png.headers()["content-type"], "image/png");
    assert_eq!(&png.bytes().await.unwrap()[..8], b"\x89PNG\r\n\x1a\n");
}
