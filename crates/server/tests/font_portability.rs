#[allow(dead_code)]
mod support;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use support::Server;

const LATO: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");
const NOTO: &[u8] = include_bytes!("../../../renderer/fonts/NotoSans.ttf");

fn body(revision: u64, id: &str, bytes: &[u8]) -> Value {
    json!({"expectedRevision":revision,"idempotencyKey":id,"mediaType":"font/ttf",
        "base64":STANDARD.encode(bytes),"font":{"id":id,"family":"Shared Family","weight":400,"style":"normal","license":"OFL-1.1"}})
}

async fn import(s: &Server, p: &str, body: &Value) -> Value {
    s.client
        .post(format!("{}/api/v1/projects/{p}/fonts", s.url))
        .header("origin", &s.url)
        .json(body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[tokio::test]
async fn exact_downloads_metadata_replay_atomicity_and_http_guards() {
    let s = Server::start().await;
    let p = s.project().await;
    let request = body(0, "lato", LATO);
    let mut events = s.state.events.subscribe();
    let created = import(&s, &p, &request).await;
    assert_eq!(created["committedRevision"], 1, "{created}");
    assert_eq!(events.recv().await.unwrap()["revision"], 1);
    assert_eq!(import(&s, &p, &request).await["replayed"], true);
    assert!(matches!(
        events.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    assert_eq!(
        import(&s, &p, &body(0, "stale", NOTO)).await["error"]["code"],
        "REVISION_CONFLICT"
    );
    assert_eq!(
        import(&s, &p, &body(1, "bad", b"invalid")).await["error"]["code"],
        "VALIDATION_FAILED"
    );
    let list = s.get(&format!("/api/v1/projects/{p}/fonts")).await;
    assert_eq!(list["revision"], 1);
    assert_eq!(list["fonts"].as_array().unwrap().len(), 2);
    for (id, bytes, source) in [("lato", LATO, "project"), ("font_bundled", NOTO, "bundled")] {
        let font = list["fonts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == id)
            .unwrap();
        assert_eq!(font["source"], source);
        assert_eq!(font["mediaType"], "font/ttf");
        assert_eq!(font["byteSize"], bytes.len());
        let response = s
            .client
            .get(font["downloadUrl"].as_str().unwrap())
            .header("origin", "http://127.0.0.1:54321")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-type"], "font/ttf");
        assert_eq!(
            response.headers()["access-control-allow-origin"],
            "http://127.0.0.1:54321"
        );
        assert_eq!(
            response.headers()["content-disposition"],
            format!("attachment; filename=\"{id}.ttf\"")
        );
        assert_eq!(response.bytes().await.unwrap().as_ref(), bytes);
    }
    let mcp = s.call("list_fonts", json!({"projectId":p,"limit":1})).await;
    assert!(mcp["items"][0]["downloadUrl"].is_string());
    assert_eq!(mcp["truncated"], true);
    let url = format!("{}/api/v1/projects/{p}/fonts", s.url);
    for origin in ["http://evil.example", "http://127.0.0.1:54321"] {
        let rejected = s
            .client
            .post(&url)
            .header("origin", origin)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(rejected.status(), 403);
    }
    assert_eq!(
        s.client
            .get(&url)
            .header("host", "evil.example")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        s.get(&format!("/api/v1/projects/{p}/fonts")).await["revision"],
        1
    );
    assert_eq!(
        s.client
            .get(format!("{url}/unknown/download"))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let copied = s
        .call("create_project", json!({"slug":"copied","name":"Copied"}))
        .await;
    let second = copied["projectId"].as_str().unwrap();
    assert_eq!(
        import(&s, second, &body(0, "copy", LATO)).await["committedRevision"],
        1
    );
    let response = s
        .client
        .get(format!(
            "{}/api/v1/projects/{second}/fonts/copy/download",
            s.url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.bytes().await.unwrap().as_ref(), LATO);
}

#[tokio::test]
async fn svg_embeds_exact_fonts_and_loads_offline_without_installed_families() {
    let s = Server::start().await;
    let p = s.project().await;
    assert_eq!(
        import(&s, &p, &body(0, "noto", NOTO)).await["committedRevision"],
        1
    );
    assert_eq!(
        import(&s, &p, &body(1, "lato", LATO)).await["committedRevision"],
        2
    );
    let mut design = support::design();
    design[2]["props"]["fontId"] = json!("noto");
    let mut second = design[2].clone();
    second["id"] = json!("other");
    second["props"]["fontId"] = json!("lato");
    second["props"]["y"] = json!(60);
    design.as_array_mut().unwrap().push(second);
    assert_eq!(
        s.batch(&p, 2, "design", design).await["committedRevision"],
        3
    );
    let exports = s
        .call(
            "export_nodes",
            json!({"projectId":p,"nodeIds":["frame"],"settings":[{"format":"svg"}]}),
        )
        .await;
    let item = &exports["exports"][0];
    assert_eq!(item["status"], "ready", "{exports}");
    let svg = s
        .client
        .get(item["files"][0]["url"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(svg.contains(&STANDARD.encode(NOTO)));
    assert!(svg.contains(&STANDARD.encode(LATO)));
    assert!(
        !svg.contains("Shared Family") && !svg.contains("sans-serif") && !svg.contains("/api/v1/")
    );
    let path = s.root.path().join("portable.svg");
    std::fs::write(&path, &svg).unwrap();
    let output = std::process::Command::new("node")
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args([
            "spikes/pipeline/font-svg-offline.mjs",
            path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
