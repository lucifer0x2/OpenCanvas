use super::*;
#[test]
fn actual_sqlite_commit_reuses_topology_and_lazy_read_dependencies() {
    let temp = tempfile::tempdir().unwrap();
    let storage: Storage = Store::open(temp.path()).unwrap().into();
    let p = storage.create_project("topology", "Topology").unwrap();
    let mut request = BatchRequest { project_id:p.id().into(), expected_revision:0, idempotency_key:"seed".into(), operations:serde_json::from_value(json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":100,"height":100}},
        {"op":"create_node","id":"rect","kind":"rectangle","parentId":"frame","props":{"width":10,"height":10}}
    ])).unwrap() };
    p.apply_batch(request.clone()).unwrap();
    let old = storage.shared_snapshot(p.id()).unwrap();
    assert!(old.indexes.dependencies.lock().unwrap().is_none());
    old.edit_context(vec!["page".into()]).unwrap();
    let dependency = old
        .indexes
        .dependencies(|| panic!("warm read rebuilt dependencies"))
        .unwrap();
    let again = storage.shared_snapshot(p.id()).unwrap();
    assert!(Arc::ptr_eq(
        &dependency,
        &again
            .indexes
            .dependencies(|| panic!("read index not reused"))
            .unwrap()
    ));
    request.expected_revision = 1;
    request.idempotency_key = "sparse".into();
    request.operations = vec![Operation::UpdateNode {
        id: "rect".into(),
        props: json!({"width":11}),
    }];
    p.apply_batch(request).unwrap();
    let new = storage.shared_snapshot(p.id()).unwrap();
    assert!(!Arc::ptr_eq(&old.indexes, &new.indexes));
    assert!(
        Arc::ptr_eq(&old.indexes.topology, &new.indexes.topology),
        "commit must reuse the read's cached topology"
    );
    assert!(Arc::ptr_eq(&old.indexes.pages, &new.indexes.pages));
    assert!(
        new.indexes.dependencies.lock().unwrap().is_none(),
        "strict commit must not expand dependency work"
    );
}

#[test]
fn sqlite_readiness_identity_never_becomes_raw_write_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let p = store.create_project("readiness", "Readiness").unwrap();
    p.import_asset(ImportAssetRequest {
        project_id: p.id().into(),
        expected_revision: 0,
        idempotency_key: "font-bytes".into(),
        media_type: "font/ttf".into(),
        bytes: include_bytes!("../../../../renderer/fonts/NotoSans.ttf").to_vec(),
        font: None,
    })
    .unwrap();
    let storage: Storage = store.into();
    let ready = storage.shared_snapshot(p.id()).unwrap();
    let asset = &ready.snapshot.assets[0];
    fs::remove_file(p.asset_path(&asset.relative_key).unwrap()).unwrap();
    let missing = storage.shared_snapshot(p.id()).unwrap();
    assert_eq!(ready.snapshot.revision, missing.snapshot.revision);
    assert_eq!(missing.snapshot.assets[0].status, "missing");
    assert!(!Arc::ptr_eq(&ready.indexes, &missing.indexes));
    p.apply_batch(BatchRequest {
        project_id: p.id().into(),
        expected_revision: 1,
        idempotency_key: "write-raw".into(),
        operations: vec![Operation::CreateNode {
            id: "page".into(),
            kind: NodeKind::Page,
            parent_id: None,
            props: json!({}),
            index: None,
        }],
    })
    .unwrap();
    let raw = read_snapshot(&mut p.reader().unwrap()).unwrap();
    assert_eq!(
        raw.assets[0].status, "ready",
        "public readiness must not be persisted"
    );
}

#[test]
#[cfg(unix)]
fn replaced_sqlite_file_with_same_id_and_revision_reopens_writer_and_indexes() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let p = store.create_project("replaced", "Replaced").unwrap();
    let storage: Storage = store.into();
    let old = storage.shared_snapshot(p.id()).unwrap();
    p.checkpoint().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let displaced = outside.path().join("displaced");
    fs::rename(&p.path, &displaced).unwrap();
    fs::create_dir(&p.path).unwrap();
    fs::create_dir(p.path.join("assets")).unwrap();
    fs::create_dir(p.path.join("inbox")).unwrap();
    fs::copy(displaced.join("db.sql"), p.path.join("db.sql")).unwrap();
    let replacement = Connection::open(p.path.join("db.sql")).unwrap();
    replacement
        .execute("UPDATE project_meta SET name='New file'", [])
        .unwrap();
    drop(replacement);
    let new = storage.shared_snapshot(p.id()).unwrap();
    assert_eq!(new.snapshot.revision, old.snapshot.revision);
    assert_eq!(new.snapshot.name, "New file");
    assert!(!Arc::ptr_eq(&old.indexes, &new.indexes));
    assert!(matches!(p.reader(), Err(error) if error.code=="PROJECT_REPLACED"));
    let handle = storage.open_project(p.id()).unwrap();
    handle
        .apply_batch(BatchRequest {
            project_id: p.id().into(),
            expected_revision: 0,
            idempotency_key: "replacement-write".into(),
            operations: vec![Operation::CreateNode {
                id: "page".into(),
                kind: NodeKind::Page,
                parent_id: None,
                props: json!({}),
                index: None,
            }],
        })
        .unwrap();
    assert_eq!(
        storage
            .shared_snapshot(p.id())
            .unwrap()
            .snapshot
            .nodes
            .len(),
        1
    );
}
