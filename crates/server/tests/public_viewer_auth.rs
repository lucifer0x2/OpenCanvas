#[allow(dead_code)]
mod public_support;
#[allow(dead_code)]
mod support;

use opencanvas_server::PublicViewer;
use public_support::{PublicServer, HOST, ORIGIN};
use reqwest::{Method, RequestBuilder, Response};
use serde_json::{json, Value};
use std::time::Duration;
use support::Server;

#[path = "public_viewer_auth/proxy.rs"]
mod proxy;

const AUTH: &str = "/api/v1/viewer/auth";
const PROJECTS: &str = "/api/v1/projects";
const PASSWORD: &str = "a browser test passphrase";
const SET: &str = "set_public_viewer_password";
const LIST: &str = "list_public_viewer_sessions";
const REVOKE: &str = "revoke_public_viewer_session";

fn login(public: &PublicServer, password: &str) -> RequestBuilder {
    public
        .request(Method::POST, AUTH)
        .header("user-agent", "Viewer auth fixture")
        .json(&json!({"password":password}))
}

async fn cookie(response: Response) -> String {
    assert_eq!(response.status(), 200);
    let value = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(value.starts_with("__Host-opencanvas_viewer="));
    for attribute in [
        "Secure",
        "HttpOnly",
        "SameSite=Strict",
        "Path=/",
        "Max-Age=7776000",
    ] {
        assert!(
            value.split("; ").any(|part| part == attribute),
            "Missing cookie attribute {attribute}"
        );
    }
    assert!(!value.contains("Domain="));
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({"authenticated":true,"required":true})
    );
    value.split_once(';').unwrap().0.into()
}

#[tokio::test]
async fn only_public_api_data_requires_the_cookie_and_login_is_origin_checked_and_throttled() {
    let local = Server::start().await;
    let public = PublicServer::start(local.state.clone()).await;
    assert_eq!(
        public.get(AUTH).await,
        json!({"authenticated":true,"required":false})
    );
    let anonymous = public.request(Method::GET, PROJECTS).send().await.unwrap();
    assert_eq!(anonymous.status(), 200);
    assert_eq!(anonymous.headers()["cache-control"], "no-store");
    let project = local.project().await;
    assert_eq!(
        local.call(SET, json!({})).await["error"]["code"],
        "INVALID_REQUEST"
    );
    assert_eq!(
        local.call(SET, json!({"password":PASSWORD})).await,
        json!({"required":true,"revokedSessions":0})
    );
    assert_eq!(
        public.get(AUTH).await,
        json!({"authenticated":false,"required":true})
    );
    assert_eq!(
        local.get(AUTH).await,
        json!({"authenticated":true,"required":false})
    );
    assert_eq!(
        local.get(PROJECTS).await["projects"][0]["projectId"],
        project
    );
    assert_eq!(public.get("/api/v1/viewer").await, json!({"readOnly":true}));
    assert_ne!(
        public
            .request(Method::GET, "/")
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        local
            .client
            .post(format!("{}{AUTH}", local.url))
            .send()
            .await
            .unwrap()
            .status(),
        405
    );

    for path in [
        "/events",
        "/projects",
        "/projects/p/snapshot",
        "/projects/p/events",
        "/projects/p/assets/a",
        "/projects/p/scene",
        "/projects/p/fonts",
        "/projects/p/fonts/f/download",
        "/fonts/font_bundled",
        "/jobs/j",
        "/jobs/j/image",
        "/jobs/j/files/0",
    ] {
        let response = public
            .request(Method::GET, &format!("/api/v1{path}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 401, "{path}");
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    for (method, path, status) in [
        (Method::POST, "/api/v1/projects/p/exports", 401),
        (Method::HEAD, PROJECTS, 401),
        (Method::POST, "/api/v1/viewer/auth/", 403),
        (Method::POST, "/mcp", 403),
    ] {
        assert_eq!(
            public.request(method, path).send().await.unwrap().status(),
            status
        );
    }
    for origin in [None, Some("https://example.com"), Some("null")] {
        let mut request = public
            .client
            .post(format!("{}{AUTH}", public.url))
            .header("host", HOST);
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        assert_eq!(
            request
                .json(&json!({"password":PASSWORD}))
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
    }
    let oversize = format!("{}{}", json!({"password":PASSWORD}), " ".repeat(16 * 1024));
    assert_eq!(
        public
            .request(Method::POST, AUTH)
            .header("content-type", "application/json")
            .body(oversize)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(login(&public, "wrong").send().await.unwrap().status(), 401);
    let cookie = cookie(
        login(&public, PASSWORD)
            .header("x-forwarded-for", "203.0.113.9")
            .send()
            .await
            .unwrap(),
    )
    .await;
    let response = public
        .request(Method::GET, PROJECTS)
        .header("cookie", &cookie)
        .header("x-forwarded-for", "203.0.113.10")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(
        response.json::<Value>().await.unwrap()["projects"],
        json!([])
    );
    let metadata = format!("{PROJECTS}/{project}");
    let authorized_project = || {
        public
            .request(Method::GET, &metadata)
            .header("cookie", &cookie)
    };
    assert_eq!(authorized_project().send().await.unwrap().status(), 404);
    public_support::set_privacy(&local, &project, "public").await;
    let shared = authorized_project().send().await.unwrap();
    assert_eq!(shared.status(), 200);
    assert_eq!(
        shared.json::<Value>().await.unwrap()["privacy_status"],
        "public"
    );
    assert_eq!(
        public
            .request(Method::PATCH, &metadata)
            .header("cookie", &cookie)
            .json(&json!({"privacy_status":"private"}))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    public_support::set_privacy(&local, &project, "private").await;
    assert_eq!(authorized_project().send().await.unwrap().status(), 404);
    let list = local.call(LIST, json!({})).await;
    assert_eq!(list["sessions"][0]["ipAddress"], "127.0.0.1");
    assert_eq!(list["sessions"][0]["userAgent"], "Viewer auth fixture");
    for _ in 0..3 {
        assert_eq!(login(&public, "wrong").send().await.unwrap().status(), 401);
    }
    let limited = login(&public, PASSWORD).send().await.unwrap();
    assert_eq!(limited.status(), 429);
    assert_eq!(limited.headers()["retry-after"], "60");
}
