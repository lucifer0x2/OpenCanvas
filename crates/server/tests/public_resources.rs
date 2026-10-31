mod public_support;
#[allow(dead_code)]
mod support;
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::StreamExt;
use public_support::PublicServer;
use reqwest::Method;
use serde_json::{json, Value};
use std::time::Duration;
use support::Server;

const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=";
const FONT: &[u8] = include_bytes!("../../../renderer/fonts/NotoSans.ttf");

#[tokio::test]
async fn public_resources_exports_and_events_share_state_but_keep_worker_callbacks_local() {
    let local = Server::start().await;
    let public = PublicServer::start(local.state.clone()).await;
    let project = local.project().await;
    let base = format!("/api/v1/projects/{project}");
    let imported = local.client.post(format!("{}{base}/fonts", local.url))
        .json(&json!({"expectedRevision":0,"idempotencyKey":"font-local","mediaType":"font/ttf","base64":STANDARD.encode(FONT),"font":{"id":"public_font","family":"Public font","weight":400,"style":"normal"}}))
        .send().await.unwrap();
    assert_eq!(imported.status(), 200, "Local font imports preserved");
    let imported = local.call("import_asset", json!({"projectId":project,"expectedRevision":1,"idempotencyKey":"image-local","mediaType":"image/png","base64":PNG})).await;
    assert!(imported.get("error").is_none(), "{imported}");
    let snapshot = public.get(&format!("{base}/snapshot")).await;
    let asset = snapshot["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["mediaType"] == "image/png")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let mut ops = support::design();
    ops[2]["props"]["fontId"] = json!("public_font");
    // Authored content resembling a callback URL must never be rewritten.
    ops[2]["props"]["name"] = json!(format!("{}/api/v1/authored", local.url));
    ops.as_array_mut().unwrap().push(json!({"op":"create_node","id":"image","kind":"image","parentId":"frame","props":{"assetId":asset,"x":200,"y":100,"width":32,"height":32}}));
    assert_eq!(
        local.batch(&project, 2, "design", ops).await["committedRevision"],
        3
    );
    let fonts = public.get(&format!("{base}/fonts")).await;
    for font in fonts["fonts"].as_array().unwrap() {
        assert_eq!(
            public
                .resource(&font["downloadUrl"])
                .await
                .bytes()
                .await
                .unwrap()
                .as_ref(),
            FONT
        );
    }
    let response = public
        .request(Method::GET, &format!("{base}/assets/{asset}"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.bytes().await.unwrap().as_ref(),
        STANDARD.decode(PNG).unwrap()
    );
    assert_eq!(
        public
            .request(Method::GET, "/api/v1/fonts/font_bundled")
            .send()
            .await
            .unwrap()
            .status(),
        200
    );

    let mut job = public
        .get(&format!(
            "{base}/scene?frameId=frame&revision=3&width=320&height=200"
        ))
        .await;
    let id = job["jobId"].as_str().unwrap().to_owned();
    for _ in 0..400 {
        if job["status"] != "pending" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        job = public.get(&format!("/api/v1/jobs/{id}")).await;
    }
    assert_eq!(job["status"], "ready", "{job}");
    assert!(job.get("error").is_none(), "{job}");
    for key in ["fonts", "assets"] {
        assert!(!job["scene"][key].as_array().unwrap().is_empty());
        for resource in job["scene"][key].as_array().unwrap() {
            public.resource(&resource["url"]).await;
        }
    }
    assert_eq!(
        public.resource(&job["imageUrl"]).await.headers()["content-type"],
        "image/png"
    );
    assert_eq!(job["image"]["uri"], job["imageUrl"]);
    let internal = local.get(&format!("/api/v1/jobs/{id}")).await;
    assert!(internal["scene"]["fonts"][0]["url"]
        .as_str()
        .unwrap()
        .starts_with(&local.url));
    assert!(internal["imageUrl"]
        .as_str()
        .unwrap()
        .starts_with(&local.url));
    assert_eq!(local.state.config.base_url, local.url);

    let exported = public.request(Method::POST, &format!("{base}/exports"))
        .json(&json!({"nodeIds":["frame"],"settings":[{"format":"svg"},{"format":"png"}],"revision":3,"waitMs":10000}))
        .send().await.unwrap();
    assert_eq!(exported.status(), 200);
    let exported: Value = exported.json().await.unwrap();
    let item = &exported["exports"][0];
    assert_eq!(item["status"], "ready", "{exported}");
    let files = item["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    for file in files {
        let response = public.resource(&file["url"]).await;
        assert!(response.headers()["content-disposition"]
            .to_str()
            .unwrap()
            .starts_with("attachment"));
        let bytes = response.bytes().await.unwrap();
        assert!(!bytes.is_empty());
        if file["format"] == "svg" {
            let svg = String::from_utf8(bytes.to_vec()).unwrap();
            assert!(svg.contains("data:font/ttf;base64,"));
            assert!(svg.contains("data:image/png;base64,"));
        }
    }
    assert_eq!(public.get(&format!("{base}/snapshot")).await["revision"], 3);

    // Both SSE surfaces see writes made through local MCP and end on shared shutdown.
    let stream = public
        .request(Method::GET, &format!("{base}/events?cursor={project}:3"))
        .send()
        .await
        .unwrap();
    assert_eq!(stream.status(), 200);
    let mut stream = stream.bytes_stream();
    let workspace = public
        .request(Method::GET, "/api/v1/events")
        .send()
        .await
        .unwrap();
    assert_eq!(workspace.status(), 200);
    let mut workspace = workspace.bytes_stream();
    assert_eq!(
        local
            .batch(
                &project,
                3,
                "live",
                json!([{"op":"update_node","id":"text","props":{"text":"Live public update"}}])
            )
            .await["committedRevision"],
        4
    );
    let event = tokio::time::timeout(Duration::from_secs(5), async {
        let mut text = String::new();
        while !text.contains("event: commit") {
            text.push_str(&String::from_utf8_lossy(
                &stream.next().await.unwrap().unwrap(),
            ));
        }
        text
    })
    .await
    .unwrap();
    assert!(event.contains("\"revision\":4"), "{event}");
    let event = tokio::time::timeout(Duration::from_secs(5), workspace.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(String::from_utf8_lossy(&event).contains("project_activity"));
    local.state.begin_shutdown();
    tokio::time::timeout(Duration::from_secs(5), async {
        while stream.next().await.is_some() {}
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while workspace.next().await.is_some() {}
    })
    .await
    .unwrap();
}
