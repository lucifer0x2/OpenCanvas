#[path = "project_privacy/inflight.rs"]
mod inflight;
#[allow(dead_code)]
mod public_support;
#[allow(dead_code)]
mod support;

#[path = "project_privacy/stream.rs"]
mod stream;

use public_support::{set_privacy, PublicServer};
use reqwest::Method;
use serde_json::{json, Value};
use std::time::Duration;
use support::Server;

const PROJECTS: &str = "/api/v1/projects";

#[tokio::test]
async fn an_existing_database_without_privacy_is_private_on_both_listeners() {
    let local = Server::start().await;
    let project = local
        .state
        .store
        .create_project("legacy", "Legacy project")
        .unwrap();
    let id = project.id().to_owned();
    drop(project);
    let db = rusqlite::Connection::open(local.root.path().join("legacy/db.sql")).unwrap();
    db.execute_batch(
        "ALTER TABLE project_meta DROP COLUMN privacy_status;
         UPDATE project_meta SET schema_version=4 WHERE singleton=1;
         PRAGMA user_version=4;",
    )
    .unwrap();
    drop(db);

    let public = PublicServer::start(local.state.clone()).await;
    assert_eq!(public.get(PROJECTS).await["projects"], json!([]));
    assert_eq!(
        local.get(PROJECTS).await["projects"][0]["privacy_status"],
        "private"
    );
    let path = format!("{PROJECTS}/{id}/snapshot");
    assert_eq!(
        public
            .request(Method::GET, &path)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(local.get(&path).await["privacy_status"], "private");
}

#[tokio::test]
async fn defaults_filter_before_paging_and_update_without_revising_the_snapshot() {
    let local = Server::start().await;
    let public = PublicServer::start(local.state.clone()).await;
    for (slug, privacy) in [
        ("a-hidden", "private"),
        ("b-shared", "public"),
        ("c-hidden", "private"),
        ("d-shared", "public"),
        ("e-hidden", "private"),
    ] {
        let created = local
            .call("create_project", json!({"slug":slug,"name":slug}))
            .await;
        assert_eq!(created["privacy_status"], "private");
        if privacy == "public" {
            set_privacy(&local, slug, privacy).await;
        }
    }
    let first = public.get(&format!("{PROJECTS}?limit=1")).await;
    assert_eq!(first["nextCursor"], "b-shared");
    assert_eq!(first["projects"][0]["slug"], "b-shared");
    assert_eq!(first["projects"][0]["privacy_status"], "public");
    let second = public
        .get(&format!("{PROJECTS}?limit=1&cursor=b-shared"))
        .await;
    assert_eq!(second["projects"][0]["slug"], "d-shared");
    assert_eq!(second["nextCursor"], Value::Null);
    assert_eq!(
        local.get(PROJECTS).await["projects"]
            .as_array()
            .unwrap()
            .len(),
        5
    );

    let private = local.get(&format!("{PROJECTS}/a-hidden")).await;
    let id = private["projectId"].as_str().unwrap();
    assert_eq!(private["privacy_status"], "private");
    for suffix in ["", "/snapshot", "/events"] {
        let mut bodies = Vec::new();
        for key in ["a-hidden", id, "unknown"] {
            let response = public
                .request(Method::GET, &format!("{PROJECTS}/{key}{suffix}"))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 404);
            bodies.push(response.json::<Value>().await.unwrap());
        }
        assert_eq!(bodies[0], bodies[1]);
        assert_eq!(bodies[0], bodies[2]);
    }
    let update = |body: Value| {
        local
            .client
            .patch(format!("{}{PROJECTS}/a-hidden", local.url))
            .json(&body)
    };
    assert_eq!(
        update(json!({"privacy_status":"unknown"}))
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        update(json!({"privacy_status":"public","name":"altered"}))
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    let mut snapshots = Vec::new();
    for suffix in ["", "?nodeFormat=rows"] {
        let url = format!("{}{PROJECTS}/{id}/snapshot{suffix}", local.url);
        let response = local.client.get(&url).send().await.unwrap();
        let etag = response.headers()["etag"].clone();
        assert_eq!(
            response.json::<Value>().await.unwrap()["privacy_status"],
            "private"
        );
        snapshots.push((url, etag));
    }
    let published = update(json!({"privacy_status":"public"}))
        .send()
        .await
        .unwrap();
    assert_eq!(published.status(), 200);
    assert_eq!(
        published.json::<Value>().await.unwrap()["privacy_status"],
        "public"
    );
    for (url, old_etag) in snapshots {
        let current = local
            .client
            .get(url)
            .header("if-none-match", &old_etag)
            .send()
            .await
            .unwrap();
        assert_eq!(current.status(), 200);
        assert_ne!(current.headers()["etag"], old_etag);
        let snapshot: Value = current.json().await.unwrap();
        assert_eq!(snapshot["privacy_status"], "public");
        assert_eq!(snapshot["revision"], private["revision"]);
    }
    assert_eq!(
        public.get(&format!("{PROJECTS}/a-hidden")).await["projectId"],
        id
    );
    let current = public
        .request(Method::GET, &format!("{PROJECTS}/{id}/snapshot"))
        .send()
        .await
        .unwrap();
    let public_etag = current.headers()["etag"].clone();
    set_privacy(&local, id, "private").await;
    let hidden = public
        .request(Method::GET, &format!("{PROJECTS}/{id}/snapshot"))
        .header("if-none-match", public_etag)
        .send()
        .await
        .unwrap();
    assert_eq!(hidden.status(), 404);
}
