mod postgres_backend_support;
use base64::{engine::general_purpose::STANDARD, Engine};
use postgres_backend_support::Server;
use serde_json::json;

const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
async fn injected_postgres_serves_real_http_mcp_resources_history_and_reconnect_replay() {
    let server = Server::start().await;
    let slug = format!("http-pg-{}", uuid::Uuid::new_v4().simple());
    let response = server
        .client
        .post(format!("{}/api/v1/projects", server.url))
        .json(&json!({"slug":slug,"name":"Runtime PostgreSQL"}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let created: serde_json::Value = response.json().await.unwrap();
    let id = created["projectId"].as_str().unwrap().to_owned();
    let listed = server.call("list_projects", json!({})).await;
    assert!(listed["projects"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["projectId"] == id));
    let import = json!({"projectId":id,"expectedRevision":0,"idempotencyKey":"font","mediaType":"font/ttf","base64":STANDARD.encode(FONT),"font":{"id":"lato","family":"Lato","weight":400,"style":"normal","license":"OFL-1.1"}});
    let imported = server.call("import_asset", import.clone()).await;
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    let asset_id = imported["asset"]["id"].as_str().unwrap();
    let original = json!({"projectId":id,"expectedRevision":1,"idempotencyKey":"page","operations":[{"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"name":"Actual PG"}},{"op":"create_note","id":"note","nodeId":"page","text":"MCP note"}]});
    let applied = server.call("apply_batch", original.clone()).await;
    assert_eq!(applied["committedRevision"], 2, "{applied}");
    let dry = server.call("apply_batch", json!({"projectId":id,"expectedRevision":2,"idempotencyKey":"preview","dryRun":true,"operations":[{"op":"update_node","id":"page","props":{"name":"Preview"}}]})).await;
    assert_eq!(dry["dryRun"], true, "{dry}");
    let invalid = server.call("apply_batch", json!({"projectId":id,"expectedRevision":2,"idempotencyKey":"invalid","operations":[{"op":"update_node","id":"page","props":{"name":"Rollback"}},{"op":"update_node","id":"absent","props":{"name":"Fail"}}]})).await;
    assert_eq!(invalid["error"]["code"], "VALIDATION_FAILED", "{invalid}");
    let snapshot = server
        .get(&format!("/api/v1/projects/{slug}/snapshot"))
        .await;
    assert_eq!(
        server
            .get(&format!("/api/v1/projects/{slug}/snapshot"))
            .await,
        snapshot
    );
    assert_eq!(snapshot["revision"], 2);
    assert_eq!(snapshot["nodes"][0]["props"]["name"], "Actual PG");
    assert_eq!(snapshot["notes"][0]["text"], "MCP note");
    let changes = server
        .call(
            "get_changes",
            json!({"projectId":id,"sinceRevision":0,"includeOperations":true}),
        )
        .await;
    assert_eq!(changes["currentRevision"], 2);
    assert_eq!(changes["commits"].as_array().unwrap().len(), 2);
    let ahead = server
        .call("get_changes", json!({"projectId":id,"sinceRevision":99}))
        .await;
    assert_eq!(ahead["error"]["code"], "RESET_REQUIRED");
    let asset = server
        .client
        .get(format!(
            "{}/api/v1/projects/{id}/assets/{asset_id}",
            server.url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(asset.headers()["content-type"], "font/ttf");
    assert_eq!(asset.bytes().await.unwrap().as_ref(), FONT);
    let download = server
        .client
        .get(format!(
            "{}/api/v1/projects/{id}/fonts/lato/download",
            server.url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(download.headers()["content-type"], "font/ttf");
    assert_eq!(download.bytes().await.unwrap().as_ref(), FONT);
    let bundled = server
        .client
        .get(format!(
            "{}/api/v1/projects/{id}/fonts/font_bundled/download",
            server.url
        ))
        .send()
        .await
        .unwrap();
    assert!(bundled.status().is_success());
    assert!(bundled.bytes().await.unwrap().len() > 10_000);
    drop(server);
    let reconnected = Server::start().await;
    assert_eq!(
        reconnected
            .get(&format!("/api/v1/projects/{id}/snapshot"))
            .await,
        snapshot
    );
    let replay = reconnected.call("apply_batch", original).await;
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["committedRevision"], 2);
    let replay = reconnected.call("import_asset", import).await;
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["committedRevision"], 1);
    assert_eq!(replay["asset"], imported["asset"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
async fn snapshot_etag_rechecks_postgres_generation_and_namespace() {
    let server = Server::start().await;
    let slug = format!("http-cache-{}", uuid::Uuid::new_v4().simple());
    let project = server
        .call("create_project", json!({"slug":slug,"name":"Original"}))
        .await;
    let id = project["projectId"].as_str().unwrap().to_owned();
    let schema = project["schemaVersion"].as_u64().unwrap() as i32;
    let url = format!(
        "{}/api/v1/projects/{slug}/snapshot?nodeFormat=rows",
        server.url
    );
    let first = server.client.get(&url).send().await.unwrap();
    assert_eq!(first.status(), 200);
    let tag = first.headers()["etag"].clone();
    assert_eq!(
        first.json::<serde_json::Value>().await.unwrap()["revision"],
        0
    );

    let deleted_id = id.clone();
    control(move |sql| {
        sql.execute(
            "DELETE FROM opencanvas_runtime_v1.projects WHERE project_id=$1",
            &[&deleted_id],
        )
        .unwrap();
    })
    .await;
    let deleted = server
        .client
        .get(&url)
        .header("if-none-match", &tag)
        .send()
        .await
        .unwrap();
    assert_eq!(deleted.status(), 404);
    let recreated_id = id.clone();
    control(move |sql| {
        sql.execute(
            "INSERT INTO opencanvas_runtime_v1.projects(project_id,slug,name,schema_version,revision) VALUES($1,$2,$3,$4,0)",
            &[&recreated_id, &slug, &b"Replacement".as_slice(), &schema],
        ).unwrap();
    }).await;
    let current = server
        .client
        .get(&url)
        .header("if-none-match", &tag)
        .send()
        .await
        .unwrap();
    assert_eq!(current.status(), 200);
    let tag = current.headers()["etag"].clone();
    assert_eq!(
        current.json::<serde_json::Value>().await.unwrap()["name"],
        "Replacement"
    );

    let rotated_id = id.clone();
    let namespace: String = control(move |sql| {
        let namespace = sql
            .query_one(
                "SELECT namespace FROM opencanvas_runtime_v1.schema_registry WHERE singleton",
                &[],
            )
            .unwrap()
            .get(0);
        let mut tx = sql.transaction().unwrap();
        tx.execute(
            "UPDATE opencanvas_runtime_v1.schema_registry SET namespace=$1 WHERE singleton",
            &[&uuid::Uuid::new_v4().to_string()],
        )
        .unwrap();
        tx.execute(
            "UPDATE opencanvas_runtime_v1.projects SET name=$1 WHERE project_id=$2",
            &[&b"New namespace".as_slice(), &rotated_id],
        )
        .unwrap();
        tx.commit().unwrap();
        namespace
    })
    .await;

    let rotated = server
        .client
        .get(&url)
        .header("if-none-match", &tag)
        .send()
        .await;
    control(move |sql| {
        let mut tx = sql.transaction().unwrap();
        tx.execute(
            "UPDATE opencanvas_runtime_v1.schema_registry SET namespace=$1 WHERE singleton",
            &[&namespace],
        )
        .unwrap();
        tx.execute(
            "DELETE FROM opencanvas_runtime_v1.projects WHERE project_id=$1",
            &[&id],
        )
        .unwrap();
        tx.commit().unwrap();
    })
    .await;
    let rotated = rotated.unwrap();
    assert_eq!(rotated.status(), 200);
    assert_eq!(
        rotated.json::<serde_json::Value>().await.unwrap()["name"],
        "New namespace"
    );
}

async fn control<T: Send + 'static>(
    f: impl FnOnce(&mut postgres::Client) -> T + Send + 'static,
) -> T {
    tokio::task::spawn_blocking(move || {
        let mut client = postgres::Client::connect(
            &std::env::var("TASK19_POSTGRES_URL").unwrap(),
            postgres::NoTls,
        )
        .unwrap();
        f(&mut client)
    })
    .await
    .unwrap()
}
