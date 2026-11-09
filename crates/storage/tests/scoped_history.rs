mod scoped_support;
use opencanvas_storage::*;
use scoped_support::*;
use serde_json::{json, Value};

#[test]
fn restart_bundle_restore_and_import_preserve_scoped_replay_and_fail_closed_coverage() {
    let (temp, storage, p) = sqlite("bundle");
    seed(&p);
    strict(&p, "unrelated", vec![update("r2", json!({"name":"new"}))]);
    let req = scoped(
        &p,
        1,
        "scoped",
        &["p1"],
        vec![update("r1", json!({"name":"scoped"}))],
    );
    assert_eq!(
        p.apply_scoped_batch(req.clone())
            .unwrap()
            .committed_revision,
        3
    );
    let id = p.id().to_owned();
    drop(p);
    drop(storage);
    let store = Store::open(temp.path()).unwrap();
    let p = store.open_project(&id).unwrap();
    assert!(p.apply_scoped_batch(req.clone()).unwrap().replayed);
    let bundle = p.export_bundle().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let target = Store::open(destination.path()).unwrap();
    let restored = target.restore_bundle("restore", &bundle.bytes).unwrap();
    assert!(restored.apply_scoped_batch(req.clone()).unwrap().replayed);
    let history = restored.commits_after(0, 10).unwrap();
    assert_eq!(history[2].base_revision, 2);
    assert!(history[2].operations.is_array());
    let mut stale = req.clone();
    stale.idempotency_key = "old-guard".into();
    assert_eq!(
        restored.apply_scoped_batch(stale).unwrap_err().code,
        "REVISION_CONFLICT"
    );
    let mut fresh = req.clone();
    fresh.expected_revision = 3;
    fresh.idempotency_key = "fresh".into();
    assert_eq!(
        restored
            .apply_scoped_batch(fresh)
            .unwrap()
            .committed_revision,
        4
    );
    // Re-exporting scrubbed history must still pass strict replay validation.
    assert!(restored.export_bundle().is_ok());
    let imported = target.import_bundle("import", &bundle.bytes).unwrap();
    let mut fresh = req;
    fresh.project_id = imported.id().into();
    fresh.expected_revision = 0;
    assert_eq!(
        imported
            .apply_scoped_batch(fresh)
            .unwrap()
            .committed_revision,
        1
    );
}

#[test]
fn missing_legacy_corrupt_and_gapped_history_never_authorize_stale_page_edits() {
    let (temp, _storage, p) = sqlite("history");
    seed(&p);
    strict(&p, "unrelated", vec![update("r2", json!({"name":"new"}))]);
    let conn = rusqlite::Connection::open(temp.path().join("history/db.sql")).unwrap();
    let original: String = conn
        .query_row("SELECT ops_json FROM commits WHERE revision=2", [], |r| {
            r.get(0)
        })
        .unwrap();
    let value: Value = serde_json::from_str(&original).unwrap();
    let mut corrupt_request = value.clone();
    corrupt_request["request"] = json!({});
    let mut corrupt_ops = value.clone();
    corrupt_ops["operations"] = json!([]);
    let mut corrupt_scopes = value.clone();
    corrupt_scopes["scopes"] = json!(["garbage"]);
    for (i, body) in [
        value["operations"].clone(),
        json!({"scopeVersion":99}),
        json!({"scopeVersion":1,"scopes":["page:p2"]}),
        corrupt_request,
        corrupt_ops,
        corrupt_scopes,
    ]
    .into_iter()
    .enumerate()
    {
        conn.execute(
            "UPDATE commits SET ops_json=?1 WHERE revision=2",
            [body.to_string()],
        )
        .unwrap();
        let req = scoped(
            &p,
            1,
            &format!("bad{i}"),
            &["p1"],
            vec![update("r1", json!({"name":"bad"}))],
        );
        assert_eq!(
            p.apply_scoped_batch(req).unwrap_err().code,
            "REVISION_CONFLICT"
        );
        assert_eq!(p.snapshot().unwrap().revision, 2);
    }
    conn.execute("DELETE FROM commits WHERE revision=2", [])
        .unwrap();
    assert_eq!(
        p.apply_scoped_batch(scoped(
            &p,
            1,
            "gap",
            &["p1"],
            vec![update("r1", json!({"name":"bad"}))]
        ))
        .unwrap_err()
        .code,
        "REVISION_CONFLICT"
    );
}

#[test]
fn deletion_with_incoming_cross_page_links_requires_both_page_guards() {
    let (_temp, _storage, p) = sqlite("links");
    seed(&p);
    strict(&p,"link",serde_json::from_value(json!([
        {"op":"create_node","id":"destination","kind":"frame","parentId":"f2","props":{"width":20,"height":20}},
        {"op":"update_node","id":"r1","props":{"links":[{"to":"f2"}]}}
    ])).unwrap());
    // Root deletion is explicitly unsupported even when incoming link cleanup
    // would touch a second page; it must never leak a partial change.
    assert_eq!(
        p.apply_scoped_batch(scoped(
            &p,
            2,
            "delete",
            &["p2"],
            vec![Operation::DeleteSubtree { id: "f2".into() }]
        ))
        .unwrap_err()
        .code,
        "SCOPED_OPERATION_UNSUPPORTED"
    );
    assert_eq!(p.snapshot().unwrap().revision, 2);
}
