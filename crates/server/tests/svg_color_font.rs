//! Explicit captured-fixture regression: no implicit network in the default suite.
//! Preparation, pinned origin/license/hash and rerun command: TASK11-SERVER-FIXES.md.
#[allow(dead_code)]
mod native_support;
use base64::{engine::general_purpose::STANDARD, Engine};
use native_support::Server;
use opencanvas_server::RenderBackend;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[tokio::test]
#[ignore = "requires explicitly prepared TASK11_COLRV1_FONT and local Chromium"]
async fn valid_colrv1_svg_preserves_original_bytes_and_live_text_on_both_backends() {
    let path = std::env::var_os("TASK11_COLRV1_FONT").expect("prepare the pinned fixture first");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(bytes.len(), 1_301_060);
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "50eabcbdf1edae2de36a211cea0e257d9b7195abe1d4ee8a260d6b39d82e35ba"
    );
    for backend in [RenderBackend::Rust, RenderBackend::Chromium] {
        let s = Server::start_backend(backend).await;
        let p = s.project().await;
        let imported: Value = s
            .client
            .post(format!("{}/api/v1/projects/{p}/fonts", s.url))
            .header("origin", &s.url)
            .json(&json!({"expectedRevision":0,"idempotencyKey":"color-font",
                "mediaType":"font/ttf","base64":STANDARD.encode(&bytes),
                "font":{"id":"color","family":"Color Probe","weight":400,"style":"normal"}}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(imported["committedRevision"], 1, "{imported}");
        let ops = json!([
            {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
            {"op":"create_node","id":"frame","kind":"frame","parentId":"page",
                "props":{"width":100,"height":100}},
            {"op":"create_node","id":"text","kind":"text","parentId":"frame",
                "props":{"text":"😀","width":64,"height":64,"fontId":"color","fontSize":32,"lineHeight":40}}
        ]);
        assert_eq!(s.batch(&p, 1, ops).await["committedRevision"], 2);
        let result = s
            .call(
                "export_nodes",
                json!({"projectId":p,"nodeIds":["text"],
            "settings":[{"format":"svg"}]}),
            )
            .await;
        let item = &result["exports"][0];
        assert_eq!(item["status"], "ready", "{result}");
        let response = s
            .client
            .get(item["files"][0]["url"].as_str().unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-type"], "image/svg+xml");
        let markup = response.text().await.unwrap();
        let encoded = markup
            .split("data:font/ttf;base64,")
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap();
        assert_eq!(STANDARD.decode(encoded).unwrap(), bytes);
        let document = roxmltree::Document::parse(&markup).unwrap();
        assert!(document
            .descendants()
            .any(|n| n.has_tag_name("text") && n.text() == Some("😀")));
        let record = s
            .get(&format!("/api/v1/jobs/{}", item["jobId"].as_str().unwrap()))
            .await;
        assert!(
            record["readiness"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == "color" && r["status"] == "ready"),
            "{record}"
        );
        if backend == RenderBackend::Rust {
            for settings in [
                json!([{"scale":1}]),
                json!([{"format":"svg"},{"scale":1}]),
                json!([{"scale":1},{"format":"svg"}]),
            ] {
                let mixed = s
                    .call(
                        "export_nodes",
                        json!({"projectId":p,"nodeIds":["text"],
                    "settings":settings}),
                    )
                    .await;
                assert_eq!(mixed["exports"][0]["status"], "failed", "{mixed}");
                assert_eq!(
                    mixed["exports"][0]["error"]["code"], "UNSUPPORTED_FEATURE",
                    "{mixed}"
                );
                assert!(mixed["exports"][0].get("files").is_none());
            }
            assert!(!s.root.path().join("worker").exists());
        }
    }
}
