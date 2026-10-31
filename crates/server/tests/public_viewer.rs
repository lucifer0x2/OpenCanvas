#[allow(dead_code)]
mod public_support;
#[allow(dead_code)]
mod support;
use opencanvas_server::PublicViewer;
use public_support::{PublicServer, HOST, ORIGIN};
use reqwest::Method;
use serde_json::json;
use support::Server;

#[tokio::test]
async fn public_route_graph_never_grants_mutations_from_headers_or_path_variants() {
    let local = Server::start().await;
    let public = PublicServer::start(local.state.clone()).await;
    let project = local.project().await;
    let paths = [
        "/mcp".to_owned(),
        "/mcp/".into(),
        "/mcp/tools".into(),
        "/MCP".into(),
        "/%6dcp".into(),
        "/m%63p/".into(),
        "/mcp%2f".into(),
        "/%256dcp".into(),
        "/api/v1/projects".into(),
        "/api/v1/projects/".into(),
        "/api/v1/%70rojects".into(),
        format!("/api/v1/projects/{project}/fonts"),
        format!("/api/v1/projects/{project}/fonts/"),
        format!("/api/v1/projects/{project}/%66onts"),
        format!("/api/v1/projects/{project}/fonts%2f"),
        format!("/api/v1/projects/{project}//exports"),
        format!("/api/v1/projects/{project}/exports/"),
        format!("/api/v1/projects/{project}/%65xports"),
        "/unknown".into(),
    ];
    for path in paths {
        for host in [HOST, "localhost", "127.0.0.1:3847"] {
            for method in [
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ] {
                let response = public
                    .client
                    .request(method.clone(), format!("{}{path}", public.url))
                    .header("host", host)
                    .header(
                        "origin",
                        if host == HOST {
                            ORIGIN
                        } else {
                            local.url.as_str()
                        },
                    )
                    .header("x-forwarded-host", "localhost")
                    .header("x-forwarded-for", "127.0.0.1")
                    .header("x-forwarded-proto", "http")
                    .header("forwarded", "for=127.0.0.1;host=localhost;proto=http")
                    .json(&json!({"slug":"forbidden","name":"Forbidden"}))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status(), 403, "{method} {path} Host:{host}");
                assert_eq!(
                    response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
                    "PUBLIC_READ_ONLY"
                );
            }
        }
    }
    for path in ["/mcp", "/mcp/", "/%6dcp", "/mcp/tools", "/MCP"] {
        assert_eq!(
            public
                .request(Method::GET, path)
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
    }
    let snapshot = local
        .get(&format!("/api/v1/projects/{project}/snapshot"))
        .await;
    assert_eq!(snapshot["revision"], 0);
    assert_eq!(
        local.get("/api/v1/projects").await["projects"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn origin_host_and_capability_are_listener_specific_and_local_writes_survive() {
    let local = Server::start().await;
    let public = PublicServer::start(local.state.clone()).await;
    assert_eq!(local.get("/api/v1/viewer").await, json!({"readOnly":false}));
    assert_eq!(public.get("/api/v1/viewer").await, json!({"readOnly":true}));
    let created = local
        .client
        .post(format!("{}/api/v1/projects", local.url))
        .json(&json!({"slug":"http-local","name":"Local HTTP"}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), 200);
    let project = local.project().await; // local MCP creation still works too.
    assert_eq!(
        public.get("/api/v1/projects").await["projects"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for headers in [
        ("host", "evil.example"),
        ("host", "localhost"),
        ("origin", "https://evil.example"),
        ("origin", "http://127.0.0.1:3847"),
    ] {
        let response = public
            .client
            .get(format!("{}/api/v1/projects", public.url))
            .header("host", if headers.0 == "host" { headers.1 } else { HOST })
            .header(
                "origin",
                if headers.0 == "origin" {
                    headers.1
                } else {
                    ORIGIN
                },
            )
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 403);
    }
    assert_eq!(
        local
            .client
            .get(format!("{}/api/v1/projects", local.url))
            .header("host", HOST)
            .header("origin", ORIGIN)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    // Missing Origin is valid for navigation and CLI downloads, but Host stays checked.
    assert_eq!(
        public
            .client
            .get(format!("{}/api/v1/projects", public.url))
            .header("host", HOST)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    // Only exact POST exports is allowed; schema rejects project writes smuggled into its body.
    let response = public
        .request(Method::POST, &format!("/api/v1/projects/{project}/exports"))
        .json(&json!({"projectId":project,"slug":"forbidden"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
}

#[test]
fn public_origin_is_https_and_has_no_credentials_or_path() {
    assert_eq!(
        PublicViewer::new(&format!("{ORIGIN}/")).unwrap().origin(),
        ORIGIN
    );
    for origin in [
        "http://canvas.example.ts.net",
        "https://user@canvas.example.ts.net",
        "https://canvas.example.ts.net/path",
        "https://canvas.example.ts.net?x=1",
        "https://canvas.example.ts.net#x",
        "garbage",
    ] {
        assert!(PublicViewer::new(origin).is_err(), "{origin}");
    }
}
