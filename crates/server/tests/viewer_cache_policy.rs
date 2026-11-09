#[allow(dead_code)]
mod public_support;
#[allow(dead_code)]
mod support;
mod viewer_cache_support;

use reqwest::{header, Method, Response};
use viewer_cache_support::{Fixture, CSS, HASHED, NEW, OLD, WASM};

const FUTURE: &str = "Fri, 01 Jan 2100 00:00:00 GMT";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

fn no_store(response: &Response, status: u16) {
    assert_eq!(response.status(), status, "{}", response.url());
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert!(response.headers().get(header::LAST_MODIFIED).is_none());
    assert!(response.headers().get(header::ETAG).is_none());
    assert!(response
        .headers()
        .get_all(header::VARY)
        .iter()
        .any(|v| v == "Accept-Encoding"));
}

#[tokio::test]
async fn shells_and_stable_assets_reload_after_same_mtime_replacement_on_both_listeners() {
    let fixture = Fixture::start().await;
    for public in [false, true] {
        let response = fixture
            .request(public, Method::GET, "/")
            .send()
            .await
            .unwrap();
        no_store(&response, 200);
        assert_eq!(response.text().await.unwrap(), OLD);
    }
    fixture.replace_preserving_mtime("/index.html", NEW);
    fixture.replace_preserving_mtime(WASM, "new-wasm");
    fixture.replace_preserving_mtime("/assets/stable.js", "new-stable");
    for public in [false, true] {
        for (path, status, expected) in [
            ("/", 200, NEW),
            ("/?project=one&frame=two", 200, NEW),
            ("/index.html?release=two", 200, NEW),
            ("/preview/deep/link", 404, NEW),
            ("/assets/missing-12345678.js?v=two", 404, NEW),
            (WASM, 200, "new-wasm"),
            (
                "/scene-wasm/opencanvas_scene_bg.wasm?v=two",
                200,
                "new-wasm",
            ),
            ("/assets/stable.js", 200, "new-stable"),
        ] {
            for method in [Method::GET, Method::HEAD] {
                let response = fixture
                    .request(public, method.clone(), path)
                    .header(header::IF_MODIFIED_SINCE, FUTURE)
                    .header(header::IF_NONE_MATCH, "*")
                    .header(header::RANGE, "bytes=0-2")
                    .send()
                    .await
                    .unwrap();
                no_store(&response, status);
                assert_eq!(
                    response.text().await.unwrap(),
                    if method == Method::HEAD { "" } else { expected }
                );
            }
        }
    }
}

#[tokio::test]
async fn only_vite_js_css_bundles_are_immutable_and_keep_conditional_and_encoding_support() {
    let fixture = Fixture::start().await;
    for public in [false, true] {
        for path in [HASHED, CSS] {
            let response = fixture
                .request(public, Method::GET, &format!("{path}?v=two"))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
            let modified = response.headers()[header::LAST_MODIFIED].clone();
            assert!(!response.bytes().await.unwrap().is_empty());
            for method in [Method::GET, Method::HEAD] {
                let response = fixture
                    .request(public, method.clone(), path)
                    .header(header::IF_MODIFIED_SINCE, modified.clone())
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status(), 304);
                assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
                assert!(response.bytes().await.unwrap().is_empty());
                let response = fixture
                    .request(public, method.clone(), path)
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status(), 200);
                assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
                if method == Method::HEAD {
                    assert!(response.bytes().await.unwrap().is_empty());
                }
            }
        }
        for (encoding, bytes) in [
            ("br", "brotli-representation"),
            ("gzip", "gzip-representation"),
        ] {
            let response = fixture
                .request(public, Method::GET, HASHED)
                .header(header::ACCEPT_ENCODING, encoding)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers()[header::CONTENT_ENCODING], encoding);
            assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
            assert_eq!(response.headers()[header::VARY], "Accept-Encoding");
            assert_eq!(response.text().await.unwrap(), bytes);
            let response = fixture
                .request(public, Method::HEAD, HASHED)
                .header(header::ACCEPT_ENCODING, encoding)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers()[header::CONTENT_ENCODING], encoding);
            assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
            assert!(response.bytes().await.unwrap().is_empty());
            let response = fixture
                .request(public, Method::GET, HASHED)
                .header(header::ACCEPT_ENCODING, encoding)
                .header(header::IF_MODIFIED_SINCE, FUTURE)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 304);
            assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
            assert_eq!(response.headers()[header::VARY], "Accept-Encoding");
            assert!(response.bytes().await.unwrap().is_empty());
        }
        for path in [
            "/assets/stable.css",
            "/assets/index-toolong123.js",
            "/assets/index-1234567.js",
            "/assets/index-12345678.wasm",
            "/scene-wasm/index-12345678.js",
            "/assets/index-12345678.js.map",
        ] {
            let response = fixture
                .request(public, Method::GET, path)
                .header(header::IF_MODIFIED_SINCE, FUTURE)
                .send()
                .await
                .unwrap();
            no_store(&response, 200);
        }
    }
}

#[tokio::test]
async fn snapshot_api_keeps_its_existing_etag_policy_on_both_listeners() {
    let fixture = Fixture::start().await;
    let project = fixture.local.project().await;
    public_support::set_privacy(&fixture.local, &project, "public").await;
    for public in [false, true] {
        let policy = if public {
            "no-store"
        } else {
            "private, no-cache"
        };
        let path = format!("/api/v1/projects/{project}/snapshot");
        let response = fixture
            .request(public, Method::GET, &path)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()[header::CACHE_CONTROL], policy);
        let etag = response.headers()[header::ETAG].clone();
        let response = fixture
            .request(public, Method::GET, &path)
            .header(header::IF_NONE_MATCH, etag)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 304);
        assert_eq!(response.headers()[header::CACHE_CONTROL], policy);
        assert!(response.bytes().await.unwrap().is_empty());
    }
}
