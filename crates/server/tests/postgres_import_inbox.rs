//! PostgreSQL keeps asset bytes in the database, but `import_asset` paths still name a file
//! in the local `<data-root>/<slug>/inbox`, with the same replay as base64. Copies between
//! projects read the source through the same storage API, so they behave as on SQLite.
#[allow(dead_code)] // Only `start_with_inbox` and `call` are used here.
mod postgres_backend_support;
use base64::{engine::general_purpose::STANDARD, Engine};
use postgres_backend_support::Server;
use serde_json::json;

const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
async fn postgres_imports_inbox_paths_and_replays_them_as_base64() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::start_with_inbox(Some(root.path().into())).await;
    let slug = format!("inbox-pg-{}", uuid::Uuid::new_v4().simple());
    let created = server
        .call("create_project", json!({"slug":slug,"name":"Inbox"}))
        .await;
    let id = created["projectId"].as_str().unwrap().to_owned();
    let inbox = root.path().join(&slug).join("inbox");
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::write(inbox.join("lato.ttf"), FONT).unwrap();
    let font = json!({"id":"lato","family":"Lato","weight":400,"style":"normal"});
    let args = |source: serde_json::Value| {
        let mut args = json!({"projectId":id,"expectedRevision":0,"idempotencyKey":"font","mediaType":"font/ttf","font":font});
        args.as_object_mut()
            .unwrap()
            .extend(source.as_object().unwrap().clone());
        args
    };
    let imported = server
        .call("import_asset", args(json!({"path":"lato.ttf"})))
        .await;
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    assert_eq!(imported["asset"]["byteSize"], FONT.len());
    let replay = server
        .call(
            "import_asset",
            args(json!({"base64":STANDARD.encode(FONT)})),
        )
        .await;
    assert_eq!(replay["replayed"], true, "{replay}");
    assert_eq!(replay["asset"], imported["asset"]);
    let escape = server
        .call("import_asset", args(json!({"path":"../lato.ttf"})))
        .await;
    assert_eq!(escape["error"]["code"], "PATH_REJECTED", "{escape}");
    let mismatch = server
        .call(
            "import_asset",
            json!({"projectId":id,"expectedRevision":1,"idempotencyKey":"png","mediaType":"image/png","path":"lato.ttf"}),
        )
        .await;
    assert_eq!(mismatch["error"]["field"], "mediaType", "{mismatch}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
async fn postgres_copies_a_font_with_its_registration_between_projects() {
    let server = Server::start_with_inbox(None).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let mut ids = Vec::new();
    for name in ["brand", "site"] {
        let slug = format!("copy-{name}-{suffix}");
        let created = server
            .call("create_project", json!({"slug":slug,"name":name}))
            .await;
        ids.push(created["projectId"].as_str().unwrap().to_owned());
    }
    let font =
        json!({"id":"lato","family":"Lato","weight":400,"style":"normal","license":"OFL-1.1"});
    let imported = server.call("import_asset", json!({"projectId":ids[0],"expectedRevision":0,"idempotencyKey":"font","mediaType":"font/ttf","base64":STANDARD.encode(FONT),"font":font})).await;
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    let copy = json!({"projectId":ids[1],"expectedRevision":0,"idempotencyKey":"copy","sourceProjectId":ids[0],"fontId":"lato"});
    let copied = server.call("import_asset", copy.clone()).await;
    assert_eq!(copied["committedRevision"], 1, "{copied}");
    assert_eq!(copied["asset"], imported["asset"]);
    assert_eq!(server.call("import_asset", copy).await["replayed"], true);
    let fonts = server
        .call("list_fonts", json!({"projectId":ids[1],"ids":["lato"]}))
        .await;
    let lato = &fonts["items"][0];
    assert_eq!(
        (&lato["family"], &lato["license"]),
        (&json!("Lato"), &json!("OFL-1.1")),
        "{fonts}"
    );
}
