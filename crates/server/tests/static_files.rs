//! Exercise the real static fallback alongside API routes with files in a temporary workspace.
#[allow(dead_code)]
mod native_support;
mod static_files_support;

use futures_util::StreamExt;
use reqwest::header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, VARY};
use serde_json::{json, Value};
use static_files_support::{fixture, head, request, unpack, HTML, JS, WASM};
use std::time::Duration;

#[tokio::test]
async fn wasm_and_javascript_negotiate_precompressed_files_without_changing_mime_or_bytes() {
    let server = fixture().await;
    for (path, mime, original) in [
        ("/assets/scene.wasm", "application/wasm", WASM),
        ("/assets/scene.js", "text/javascript", JS),
    ] {
        for (accept, encoding) in [
            ("br", "br"),
            ("gzip", "gzip"),
            ("gzip;q=0.5, br;q=1", "br"),
            ("gzip;q=1, br;q=0.5", "gzip"),
        ] {
            let response = request(&server, path, accept).send().await.unwrap();
            assert_eq!(response.status(), 200, "{path}: {accept}");
            assert_eq!(response.headers()[CONTENT_TYPE], mime);
            assert_eq!(response.headers()[CONTENT_ENCODING], encoding);
            assert_eq!(response.headers()[VARY], "Accept-Encoding");
            let length = response.headers()[CONTENT_LENGTH]
                .to_str()
                .unwrap()
                .parse::<usize>()
                .unwrap();
            let encoded = response.bytes().await.unwrap();
            assert_eq!(encoded.len(), length);
            assert_ne!(encoded.as_ref(), original);
            assert_eq!(unpack(encoding, &encoded), original);

            let response = head(&server, path, accept).send().await.unwrap();
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers()[CONTENT_TYPE], mime);
            assert_eq!(response.headers()[CONTENT_ENCODING], encoding);
            assert_eq!(response.headers()[CONTENT_LENGTH], length.to_string());
            assert!(response.bytes().await.unwrap().is_empty());
        }
    }
}

#[tokio::test]
async fn uncompressed_files_and_spa_fallback_remain_available() {
    let server = fixture().await;
    for accept in ["identity", "br;q=0, gzip;q=0", "deflate"] {
        let response = request(&server, "/assets/scene.wasm", accept)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()[CONTENT_TYPE], "application/wasm");
        assert!(!response.headers().contains_key(CONTENT_ENCODING));
        assert_eq!(response.bytes().await.unwrap().as_ref(), WASM);
    }
    // A supported encoding with no prebuilt sibling falls back to the original file.
    let response = request(&server, "/assets/plain.js", "br, gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert!(!response.headers().contains_key(CONTENT_ENCODING));
    assert_eq!(response.bytes().await.unwrap().as_ref(), JS);

    // Missing variants retry another accepted encoding before using identity.
    std::fs::remove_file(server.root.path().join("web/dist/assets/scene.wasm.br")).unwrap();
    let response = request(&server, "/assets/scene.wasm", "br, gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(response.headers()[CONTENT_ENCODING], "gzip");
    assert_eq!(unpack("gzip", &response.bytes().await.unwrap()), WASM);

    for (path, status) in [("/", 200), ("/project/deep-link", 404)] {
        let response = request(&server, path, "br, gzip").send().await.unwrap();
        // Keep the existing not_found_service status while returning the SPA entry point.
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()[CONTENT_TYPE], "text/html");
        assert!(!response.headers().contains_key(CONTENT_ENCODING));
        assert_eq!(response.bytes().await.unwrap().as_ref(), HTML);
    }
}

#[tokio::test]
async fn encoded_static_files_do_not_shadow_api_or_compress_event_streams() {
    let server = fixture().await;
    let project = server.project().await;
    let response = request(&server, "/api/v1/projects", "br, gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
    assert!(!response.headers().contains_key(CONTENT_ENCODING));
    assert!(!response.headers().contains_key(VARY));
    assert!(response.text().await.unwrap().contains(&project));

    for (index, path) in [
        "/api/v1/events".to_owned(),
        format!("/api/v1/projects/{project}/events"),
    ]
    .into_iter()
    .enumerate()
    {
        let response = request(&server, &path, "br, gzip").send().await.unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()[CONTENT_TYPE], "text/event-stream");
        assert!(!response.headers().contains_key(CONTENT_ENCODING));
        assert!(!response.headers().contains_key(VARY));
        let mut stream = response.bytes_stream();
        let created = server
            .call(
                "create_project",
                json!({"slug":format!("stream-{index}"),"name":"Stream test"}),
            )
            .await;
        let event = tokio::time::timeout(Duration::from_secs(5), async {
            let mut text = String::new();
            while !text.contains("event: project_created") {
                let chunk = stream.next().await.unwrap().unwrap();
                text.push_str(&String::from_utf8_lossy(&chunk));
            }
            text
        })
        .await
        .unwrap();
        assert!(
            event.contains(created["projectId"].as_str().unwrap()),
            "{event}"
        );
    }
    let response = request(&server, "/api/v1/projects/missing", "br, gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
    assert!(!response.headers().contains_key(CONTENT_ENCODING));
    assert_eq!(
        response.json::<Value>().await.unwrap()["error"]["code"],
        "PROJECT_NOT_FOUND"
    );
}

#[tokio::test]
async fn public_viewer_serves_the_same_precompressed_wasm_with_its_host_guard() {
    let server = fixture().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = opencanvas_server::PublicViewer::new("https://canvas.example.test")
        .unwrap()
        .router(server.state.clone());
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    let client = reqwest::Client::builder()
        .no_brotli()
        .no_gzip()
        .build()
        .unwrap();
    let response = client
        .get(format!("{url}/assets/scene.wasm"))
        .header("host", "canvas.example.test")
        .header("accept-encoding", "br")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/wasm");
    assert_eq!(response.headers()[CONTENT_ENCODING], "br");
    assert_eq!(response.headers()[VARY], "Accept-Encoding");
    assert_eq!(unpack("br", &response.bytes().await.unwrap()), WASM);
    let rejected = client
        .get(format!("{url}/assets/scene.wasm"))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), 403);
    task.abort();
}
