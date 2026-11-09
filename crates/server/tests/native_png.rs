//! Native frame pixels and strict decode failures through real MCP and HTTP.
#[allow(dead_code)]
mod export_support;
#[allow(dead_code)]
mod native_support;
use base64::{engine::general_purpose::STANDARD, Engine};
use export_support::{decode, pixel};
use native_support::{design, Server};
use serde_json::{json, Value};

fn image() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[23, 200, 41, 255].repeat(4))
            .unwrap();
    }
    bytes
}

async fn imported_design(s: &Server, p: &str, bytes: &[u8]) -> Value {
    let imported = s
        .call(
            "import_asset",
            json!({"projectId":p,"expectedRevision":0,
        "idempotencyKey":"image","mediaType":"image/png","base64":STANDARD.encode(bytes)}),
        )
        .await;
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    let snapshot = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    let asset = snapshot["assets"][0]["id"].clone();
    let mut ops = design();
    ops[1]["props"]["layout"] = json!({"mode":"free"});
    ops.as_array_mut()
        .unwrap()
        .push(json!({"op":"create_node","id":"image","kind":"image",
        "parentId":"frame","props":{"x":80,"y":80,"width":32,"height":32,"assetId":asset}}));
    assert_eq!(s.batch(p, 1, ops).await["committedRevision"], 2);
    asset
}

#[tokio::test]
async fn native_text_image_and_state_frame_download_actual_dimensions_and_pixels() {
    let s = Server::start().await;
    let p = s.project().await;
    let asset = imported_design(&s, &p, &image()).await;
    let pending = s
        .call(
            "render_frame",
            json!({"projectId":p,"revision":2,"frameId":"frame",
        "viewport":{"width":320,"height":200},"state":"hover","scale":0.3}),
        )
        .await;
    let job = s.wait(pending["jobId"].as_str().unwrap()).await;
    assert_eq!(job["status"], "ready", "{job}");
    assert!(job.get("error").is_none(), "{job}");
    assert_eq!(job["image"]["provenance"]["scale"], 0.3);
    assert_eq!(
        job["image"]["provenance"]["sourceSceneKey"],
        job["sceneKey"]
    );
    assert_eq!(
        (
            job["image"]["width"].as_u64(),
            job["image"]["height"].as_u64()
        ),
        (Some(96), Some(60))
    );
    assert!(job["readiness"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == asset && r["status"] == "ready"));
    let response = s
        .client
        .get(job["imageUrl"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers()["content-type"], "image/png");
    let pixels = decode(&response.bytes().await.unwrap());
    assert_eq!((pixels.0, pixels.1), (96, 60));
    assert_eq!(pixel(&pixels, 50, 40), [171, 205, 239, 255]);
    assert_eq!(pixel(&pixels, 29, 29), [23, 200, 41, 255]);
    let exports = s
        .call(
            "export_nodes",
            json!({"projectId":p,"nodeIds":["image"],
        "settings":[{"format":"svg"},{"scale":2}]}),
        )
        .await;
    let item = &exports["exports"][0];
    assert_eq!(item["status"], "ready", "{exports}");
    let markup = s
        .client
        .get(item["files"][0]["url"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(markup.contains(&format!(
        "data:image/png;base64,{}",
        STANDARD.encode(image())
    )));
    let record = s
        .get(&format!("/api/v1/jobs/{}", item["jobId"].as_str().unwrap()))
        .await;
    assert!(record["readiness"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == asset && r["status"] == "ready"));
    assert!(!s.root.path().join("worker").exists());
}

#[tokio::test]
async fn native_frame_integer_scale_preserves_provenance_and_download_dimensions() {
    let s = Server::start().await;
    let p = s.project().await;
    let ops = json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page",
            "props":{"width":10,"height":10,"fill":"#123456"}}
    ]);
    assert_eq!(s.batch(&p, 0, ops).await["committedRevision"], 1);
    for (scale, size) in [(0.3, 3), (2.0, 20)] {
        let pending = s
            .call(
                "render_frame",
                json!({"projectId":p,"revision":1,
            "frameId":"frame","viewport":{"width":10,"height":10},"scale":scale}),
            )
            .await;
        let job = s.wait(pending["jobId"].as_str().unwrap()).await;
        assert!(job.get("error").is_none(), "{job}");
        assert_eq!(job["image"]["provenance"]["scale"], scale);
        assert_eq!(job["image"]["provenance"]["engine"], "native");
        assert_eq!(
            job["image"]["provenance"]["sourceSceneKey"],
            job["sceneKey"]
        );
        let bytes = s
            .client
            .get(job["imageUrl"].as_str().unwrap())
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        let image = decode(&bytes);
        assert_eq!((image.0, image.1), (size, size));
    }
}

#[tokio::test]
async fn bad_image_decode_retains_failed_readiness_scene_and_isolated_exports() {
    let s = Server::start().await;
    let p = s.project().await;
    // Valid import header/hash, deliberately absent compressed image payload.
    let asset = imported_design(&s, &p, &image()[..33]).await;
    let mut events = s.state.events.subscribe();
    let job = s.render(&p, 2, "hover").await;
    assert_eq!(job["status"], "ready", "{job}");
    assert_eq!(job["error"]["code"], "RESOURCE_NOT_READY");
    assert!(job.get("imageUrl").is_none());
    let unavailable = s
        .get(&format!(
            "/api/v1/jobs/{}/image",
            job["jobId"].as_str().unwrap()
        ))
        .await;
    assert_eq!(unavailable["error"]["code"], "IMAGE_NOT_READY");
    assert_eq!(job["scene"]["nodes"][0]["fill"], "#abcdef");
    let failed = job["readiness"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == asset)
        .unwrap();
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["code"], "ASSET_DECODE_FAILED");
    let mut emitted = false;
    while let Ok(event) = events.try_recv() {
        emitted |= event["event"] == "scene_ready" && event["sceneKey"] == job["sceneKey"];
    }
    assert!(emitted);
    let exports = s
        .call(
            "export_nodes",
            json!({"projectId":p,"nodeIds":["image","text"],"settings":[{"scale":1}]}),
        )
        .await;
    assert_eq!(
        exports["exports"][0]["error"]["code"], "RESOURCE_NOT_READY",
        "{exports}"
    );
    assert_eq!(exports["exports"][1]["status"], "ready", "{exports}");
    let svg = s
        .call(
            "export_nodes",
            json!({"projectId":p,"nodeIds":["image","text"],"settings":[{"format":"svg"}]}),
        )
        .await;
    assert_eq!(
        svg["exports"][0]["error"]["code"], "RESOURCE_NOT_READY",
        "{svg}"
    );
    assert_eq!(svg["exports"][1]["status"], "ready", "{svg}");
    assert!(!job.to_string().contains(s.root.path().to_str().unwrap()));
}

#[tokio::test]
async fn corrupt_font_bytes_fail_metrics_without_publishing_scene_or_png() {
    let s = Server::start().await;
    let p = s.project().await;
    let bytes = [0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0];
    let imported: Value = s
        .client
        .post(format!("{}/api/v1/projects/{p}/fonts", s.url))
        .header("origin", &s.url)
        .json(&json!({"expectedRevision":0,"idempotencyKey":"bad-font",
            "mediaType":"font/ttf","base64":STANDARD.encode(bytes),"font":{"id":"bad",
                "family":"Broken","weight":400,"style":"normal"}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    let mut ops = design();
    ops[2]["props"]["fontId"] = json!("bad");
    assert_eq!(s.batch(&p, 1, ops).await["committedRevision"], 2);
    let job = s.render(&p, 2, "hover").await;
    assert_eq!(job["status"], "failed", "{job}");
    assert_eq!(job["error"]["code"], "FONT_MISSING", "{job}");
    assert!(job.get("scene").is_none());
    assert!(job.get("imageUrl").is_none());
    assert!(!job.to_string().contains(s.root.path().to_str().unwrap()));
}
