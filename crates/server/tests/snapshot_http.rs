#[allow(dead_code)]
mod public_support;
#[allow(dead_code)]
mod support;

use public_support::{set_privacy, PublicServer};
use reqwest::{header, Method, Response, StatusCode};
use serde_json::{json, Value};
use support::{design, Server};

const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=";

fn etag(response: &Response) -> header::HeaderValue {
    response.headers()[header::ETAG].clone()
}

async fn conditional(server: &Server, url: &str, tag: &header::HeaderValue) -> Response {
    server
        .client
        .get(url)
        .header(header::IF_NONE_MATCH, tag)
        .send()
        .await
        .unwrap()
}

async fn not_modified(response: Response, tag: &header::HeaderValue, cache_control: &str) {
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(&etag(&response), tag);
    assert_eq!(response.headers()[header::CACHE_CONTROL], cache_control);
    assert!(response.bytes().await.unwrap().is_empty());
}

#[tokio::test]
async fn rows_round_trip_and_both_listeners_validate_each_representation() {
    let server = Server::start().await;
    let project = server.project().await;
    set_privacy(&server, &project, "public").await;
    assert_eq!(
        server.batch(&project, 0, "seed", design()).await["committedRevision"],
        1
    );
    let public = PublicServer::start(server.state.clone()).await;
    let path = format!("/api/v1/projects/{project}/snapshot");
    let full_url = format!("{}{path}", server.url);
    let rows_url = format!("{full_url}?legacyIgnored=yes&nodeFormat=rows");
    let full = server.client.get(&full_url).send().await.unwrap();
    assert_eq!(full.status(), StatusCode::OK);
    let full_tag = etag(&full);
    let full_bytes = full.bytes().await.unwrap();
    let full: Value = serde_json::from_slice(&full_bytes).unwrap();
    assert_eq!(full["truncated"], false);
    assert!(full.get("nodeFormat").is_none());

    let rows = conditional(&server, &rows_url, &full_tag).await;
    assert_eq!(rows.status(), StatusCode::OK);
    let rows_tag = etag(&rows);
    assert_ne!(rows_tag, full_tag);
    let rows_bytes = rows.bytes().await.unwrap();
    let mut decoded: Value = serde_json::from_slice(&rows_bytes).unwrap();
    let fields = decoded.as_object_mut().unwrap();
    assert_eq!(fields.remove("nodeFormat"), Some(json!("rows-v1")));
    assert_eq!(
        fields.remove("nodeColumns"),
        Some(json!([
            "id", "parentId", "sortKey", "kind", "props", "extra"
        ]))
    );
    for node in decoded["nodes"].as_array_mut().unwrap() {
        let row = node.as_array().unwrap();
        assert_eq!(row.len(), 5);
        *node =
            json!({"id":row[0],"parentId":row[1],"sortKey":row[2],"kind":row[3],"props":row[4]});
    }
    assert_eq!(decoded, full);

    for (url, tag, bytes) in [
        (&full_url, &full_tag, &full_bytes),
        (&rows_url, &rows_tag, &rows_bytes),
    ] {
        let head = server.client.head(url).send().await.unwrap();
        assert_eq!(head.status(), StatusCode::OK);
        assert_eq!(&etag(&head), tag);
        assert_eq!(
            head.headers()[header::CONTENT_LENGTH],
            bytes.len().to_string()
        );
        assert!(head.bytes().await.unwrap().is_empty());
        not_modified(
            server
                .client
                .head(url)
                .header(header::IF_NONE_MATCH, tag)
                .send()
                .await
                .unwrap(),
            tag,
            "private, no-cache",
        )
        .await;
    }
    let public_rows = public
        .request(Method::GET, &format!("{path}?nodeFormat=rows"))
        .header(header::IF_NONE_MATCH, "\"other,opaque\"")
        .header(
            header::IF_NONE_MATCH,
            format!("W/{}", rows_tag.to_str().unwrap()),
        )
        .send()
        .await
        .unwrap();
    not_modified(public_rows, &rows_tag, "no-store").await;
    let public_full = public
        .request(Method::HEAD, &path)
        .header(header::IF_NONE_MATCH, "*")
        .send()
        .await
        .unwrap();
    not_modified(public_full, &full_tag, "no-store").await;
    let missing = server
        .client
        .get(format!("{}/api/v1/projects/absent/snapshot", server.url))
        .header(header::IF_NONE_MATCH, "*")
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    let unknown_query = server
        .client
        .get(format!("{full_url}?nodeFormat=unsupported&old=true"))
        .send()
        .await
        .unwrap();
    assert_eq!(unknown_query.bytes().await.unwrap(), full_bytes);
}

#[tokio::test]
async fn commits_change_validators_while_retry_and_atomic_failure_preserve_them() {
    let server = Server::start().await;
    let project = server.project().await;
    let other = server.state.store.create_project("other", "Other").unwrap();
    let by_slug = format!(
        "{}/api/v1/projects/test-project/snapshot?nodeFormat=rows",
        server.url
    );
    let by_id = format!(
        "{}/api/v1/projects/{project}/snapshot?nodeFormat=rows",
        server.url
    );
    let initial = server.client.get(&by_slug).send().await.unwrap();
    let initial_tag = etag(&initial);
    not_modified(
        conditional(&server, &by_id, &initial_tag).await,
        &initial_tag,
        "private, no-cache",
    )
    .await;
    let different = server
        .client
        .get(format!(
            "{}/api/v1/projects/{}/snapshot?nodeFormat=rows",
            server.url,
            other.id()
        ))
        .header(header::IF_NONE_MATCH, &initial_tag)
        .send()
        .await
        .unwrap();
    assert_eq!(different.status(), StatusCode::OK);
    assert_ne!(etag(&different), initial_tag);

    assert_eq!(
        server.batch(&project, 0, "apply", design()).await["committedRevision"],
        1
    );
    let current = conditional(&server, &by_slug, &initial_tag).await;
    assert_eq!(current.status(), StatusCode::OK);
    let current_tag = etag(&current);
    assert_ne!(current_tag, initial_tag);
    assert_eq!(current.json::<Value>().await.unwrap()["revision"], 1);
    assert_eq!(
        server.batch(&project, 0, "apply", design()).await["replayed"],
        true
    );
    let invalid = server
        .batch(
            &project,
            1,
            "rollback",
            json!([
                {"op":"update_node","id":"page","props":{"name":"Undone"}},
                {"op":"update_node","id":"absent","props":{"name":"Missing"}}
            ]),
        )
        .await;
    assert_eq!(invalid["error"]["code"], "VALIDATION_FAILED");
    not_modified(
        conditional(&server, &by_id, &current_tag).await,
        &current_tag,
        "private, no-cache",
    )
    .await;
}

#[tokio::test]
async fn live_asset_loss_and_database_loss_are_visible_after_a_warm_read() {
    let server = Server::start().await;
    let project = server.project().await;
    let imported = server.call("import_asset", json!({
        "projectId":project,"expectedRevision":0,"idempotencyKey":"asset","mediaType":"image/png","base64":PNG
    })).await;
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    let url = format!("{}/api/v1/projects/{project}/snapshot", server.url);
    let ready = server.client.get(&url).send().await.unwrap();
    let ready_tag = etag(&ready);
    let ready = ready.json::<Value>().await.unwrap();
    assert_eq!(ready["assets"][0]["status"], "ready");
    let directory = server.root.path().join("test-project");
    let asset = directory
        .join("assets")
        .join(ready["assets"][0]["relativeKey"].as_str().unwrap());
    let saved = asset.with_extension("saved");
    std::fs::rename(&asset, &saved).unwrap();
    let missing = conditional(&server, &url, &ready_tag).await;
    assert_eq!(missing.status(), StatusCode::OK);
    assert_ne!(etag(&missing), ready_tag);
    let missing = missing.json::<Value>().await.unwrap();
    assert_eq!(missing["assets"][0]["status"], "missing");
    assert_eq!(missing["revision"], 1);
    std::fs::rename(saved, asset).unwrap();
    not_modified(
        conditional(&server, &url, &ready_tag).await,
        &ready_tag,
        "private, no-cache",
    )
    .await;

    std::fs::rename(directory.join("db.sql"), directory.join("db.saved")).unwrap();
    let gone = conditional(&server, &url, &ready_tag).await;
    assert!(!gone.status().is_success() && !gone.status().is_redirection());
    assert!(!gone.headers().contains_key(header::ETAG));
}
