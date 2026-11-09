mod bundle_support;
mod scoped_support;
use opencanvas_storage::*;
use scoped_support::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn replace_history(files: &mut std::collections::BTreeMap<String, Vec<u8>>, mut history: Value) {
    history.as_object_mut().unwrap().remove("scopeHash");
    history["scopeHash"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical_json(&history)).unwrap())
    ));
    bundle_support::edit_database(
        files,
        &format!(
            "UPDATE commits SET ops_json='{}' WHERE revision=2",
            history.to_string().replace('\'', "''")
        ),
    );
}

#[test]
fn untrusted_resealed_footprints_are_scrubbed_and_inconsistent_requests_rejected() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path()).unwrap();
    let direct = store.create_project("source", "Source").unwrap();
    let initial:Vec<Operation>=serde_json::from_value(json!([
        {"op":"create_node","id":"p1","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"f1","kind":"frame","parentId":"p1","props":{"width":40,"height":40}},
        {"op":"create_node","id":"r1","kind":"rectangle","parentId":"f1","props":{"width":20,"height":20}}
    ])).unwrap();
    direct
        .apply_batch(BatchRequest {
            project_id: direct.id().into(),
            expected_revision: 0,
            idempotency_key: "seed".into(),
            operations: initial,
        })
        .unwrap();
    direct
        .apply_batch(BatchRequest {
            project_id: direct.id().into(),
            expected_revision: 1,
            idempotency_key: "changed".into(),
            operations: vec![update("r1", json!({"name":"changed"}))],
        })
        .unwrap();
    let conn = rusqlite::Connection::open(root.path().join("source/db.sql")).unwrap();
    let text: String = conn
        .query_row("SELECT ops_json FROM commits WHERE revision=2", [], |r| {
            r.get(0)
        })
        .unwrap();
    drop(conn);
    let mut history: Value = serde_json::from_str(&text).unwrap();
    let original = bundle_support::unpack(&direct.export_bundle().unwrap().bytes);
    let mut forged = original.clone();
    history["scopes"] = json!([]);
    replace_history(&mut forged, history.clone());
    let destination = tempfile::tempdir().unwrap();
    let target = Store::open(destination.path()).unwrap();
    let restored = target
        .restore_bundle("restored", &bundle_support::pack(forged))
        .unwrap();
    let stale = ScopedBatchRequest {
        project_id: restored.id().into(),
        expected_revision: 1,
        idempotency_key: "stale".into(),
        page_ids: vec!["p1".into()],
        operations: vec![update("r1", json!({"name":"stale"}))],
    };
    assert_eq!(
        restored.apply_scoped_batch(stale).unwrap_err().code,
        "REVISION_CONFLICT"
    );
    let mut malformed = original;
    history["operations"][0]["props"]["name"] = json!("different");
    replace_history(&mut malformed, history);
    assert_eq!(
        target
            .import_bundle("invalid", &bundle_support::pack(malformed))
            .err()
            .unwrap()
            .code,
        "BUNDLE_INVALID"
    );
}
