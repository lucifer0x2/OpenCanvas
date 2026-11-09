#[allow(dead_code)]
mod public_support;
#[allow(dead_code)]
mod support;

use opencanvas_server::PublicViewer;
use public_support::{PublicServer, HOST, ORIGIN};
use reqwest::{Method, RequestBuilder};
use serde_json::{json, Value};
use support::Server;

const SECOND: &str = "https://alternate.example.invalid";
const AUTH: &str = "/api/v1/viewer/auth";
const PROJECTS: &str = "/api/v1/projects";
const PASSWORD: &str = "two independent viewer domains";

async fn public(local: &Server) -> PublicServer {
    PublicServer::start_with(
        local.state.clone(),
        PublicViewer::new(ORIGIN)
            .unwrap()
            .with_allowed_origins(SECOND)
            .unwrap(),
    )
    .await
}

fn request(public: &PublicServer, method: Method, path: &str, origin: &str) -> RequestBuilder {
    public
        .client
        .request(method, format!("{}{path}", public.url))
        .header("host", origin.strip_prefix("https://").unwrap())
        .header("origin", origin)
}

async fn login(public: &PublicServer, origin: &str) -> String {
    let response = request(public, Method::POST, AUTH, origin)
        .json(&json!({"password": PASSWORD}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let cookie = response.headers()["set-cookie"].to_str().unwrap();
    assert!(cookie.contains("Secure; HttpOnly; SameSite=Strict"));
    assert!(!cookie.contains("Domain="));
    cookie.split_once(';').unwrap().0.into()
}

#[tokio::test]
async fn both_domains_serve_only_public_projects_and_reject_cross_origin_pairs() {
    let local = Server::start().await;
    let public = public(&local).await;
    let project = local.project().await;
    for origin in [ORIGIN, SECOND] {
        let response = request(&public, Method::GET, PROJECTS, origin)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.json::<Value>().await.unwrap()["projects"],
            json!([])
        );
    }
    public_support::set_privacy(&local, &project, "public").await;
    for origin in [ORIGIN, SECOND] {
        let response = request(&public, Method::GET, PROJECTS, origin)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.json::<Value>().await.unwrap()["projects"][0]["projectId"],
            project
        );
        let response = public
            .client
            .get(format!("{}{PROJECTS}", public.url))
            .header("host", origin.strip_prefix("https://").unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(
            request(&public, Method::POST, "/mcp", origin)
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
    }
    for (host, origin) in [
        (HOST, SECOND),
        ("alternate.example.invalid", ORIGIN),
        ("alternate.example.invalid:443", SECOND),
    ] {
        for method in [Method::GET, Method::POST] {
            let path = if method == Method::POST {
                AUTH
            } else {
                PROJECTS
            };
            let response = public
                .client
                .request(method, format!("{}{path}", public.url))
                .header("host", host)
                .header("origin", origin)
                .json(&json!({"password": PASSWORD}))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 403);
        }
    }
    let response = local
        .client
        .get(format!("{}{PROJECTS}", local.url))
        .header("host", "alternate.example.invalid")
        .header("origin", SECOND)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
}

#[tokio::test]
async fn sessions_keep_primary_compatibility_and_are_isolated_by_audience() {
    let local = Server::start().await;
    local
        .call("set_public_viewer_password", json!({"password": PASSWORD}))
        .await;
    let single = PublicServer::start(local.state.clone()).await;
    let primary_cookie = login(&single, ORIGIN).await;
    let public = public(&local).await;
    let secondary_cookie = login(&public, SECOND).await;
    for (origin, cookie, expected) in [
        (ORIGIN, &primary_cookie, 200),
        (SECOND, &secondary_cookie, 200),
        (ORIGIN, &secondary_cookie, 401),
        (SECOND, &primary_cookie, 401),
    ] {
        let response = request(&public, Method::GET, PROJECTS, origin)
            .header("cookie", cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), expected, "{origin}");
        let response = request(&public, Method::GET, AUTH, origin)
            .header("cookie", cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.json::<Value>().await.unwrap()["authenticated"],
            expected == 200
        );
    }
    // A cross-domain replay must not revoke either original host session.
    for (origin, cookie) in [(ORIGIN, &primary_cookie), (SECOND, &secondary_cookie)] {
        assert_eq!(
            request(&public, Method::GET, PROJECTS, origin)
                .header("cookie", cookie)
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        assert_eq!(
            public
                .client
                .post(format!("{}{AUTH}", public.url))
                .header("host", origin.strip_prefix("https://").unwrap())
                .json(&json!({"password": PASSWORD}))
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
    }
}

#[tokio::test]
async fn export_and_font_links_use_each_requests_matched_origin() {
    let local = Server::start().await;
    let public = public(&local).await;
    let project = local.project().await;
    public_support::set_privacy(&local, &project, "public").await;
    assert_eq!(local.batch(&project, 0, "rect", json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":100,"height":100}}
    ])).await["committedRevision"], 1);
    for origin in [ORIGIN, SECOND] {
        let base = format!("/api/v1/projects/{project}");
        let fonts = request(&public, Method::GET, &format!("{base}/fonts"), origin)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();
        let fonts = fonts["fonts"].as_array().unwrap();
        assert!(!fonts.is_empty());
        for font in fonts {
            assert!(font["downloadUrl"].as_str().unwrap().starts_with(origin));
        }
        let response = request(&public, Method::POST, &format!("{base}/exports"), origin)
            .json(&json!({"nodeIds":["frame"],"settings":[{"format":"svg"}],"revision":1,"waitMs":10000}))
            .send().await.unwrap();
        assert_eq!(response.status(), 200);
        let exported = response.json::<Value>().await.unwrap();
        let item = &exported["exports"][0];
        assert_eq!(item["status"], "ready", "{exported}");
        let url = item["files"][0]["url"].as_str().unwrap();
        let path = url.strip_prefix(origin).expect("request domain export URL");
        assert_eq!(
            request(&public, Method::GET, path, origin)
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        let id = item["jobId"].as_str().unwrap();
        let local_job = local.get(&format!("/api/v1/jobs/{id}")).await;
        assert!(local_job["files"][0]["url"]
            .as_str()
            .unwrap()
            .starts_with(&local.url));
    }
}
