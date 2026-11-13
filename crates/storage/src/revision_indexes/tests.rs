use super::*;

fn snapshot() -> Snapshot {
    serde_json::from_value(json!({"schemaVersion": SCHEMA_VERSION, "projectId":"p", "slug":"p", "name":"P", "revision":1,
        "nodes":[{"id":"page","parentId":null,"sortKey":1,"kind":"page","props":{}}],
        "tokens":[],"styles":[],"fonts":[],"assets":[],"notes":[],
        "retention":{"oldestCommitRevision":1,"idempotencyPolicy":"kept"}})).unwrap()
}
fn identity(snapshot: &Snapshot) -> SnapshotCacheIdentity {
    SnapshotCacheIdentity {
        project_id: snapshot.project_id.clone(),
        schema_version: snapshot.schema_version,
        revision: snapshot.revision,
        incarnation: "db:1".into(),
        asset_readiness: vec![],
    }
}

#[test]
fn exact_identity_reuses_and_all_identity_dimensions_isolate() {
    let cache = IndexCache::default();
    let snapshot = snapshot();
    let key = identity(&snapshot);
    let first = cache.for_snapshot(&snapshot, Some(key.clone()));
    assert!(Arc::ptr_eq(
        &first,
        &cache.for_snapshot(&snapshot, Some(key.clone()))
    ));
    let mut keys = vec![];
    let mut other = key.clone();
    other.revision += 1;
    keys.push(other);
    let mut other = key.clone();
    other.schema_version += 1;
    keys.push(other);
    let mut other = key.clone();
    other.project_id = "other".into();
    keys.push(other);
    let mut other = key.clone();
    other.incarnation = "db:2".into();
    keys.push(other);
    let mut other = key.clone();
    other.asset_readiness = vec![false];
    keys.push(other);
    for key in keys {
        assert!(!Arc::ptr_eq(
            &first,
            &cache.for_snapshot(&snapshot, Some(key))
        ));
    }
    assert!(!Arc::ptr_eq(
        &cache.for_snapshot(&snapshot, None),
        &cache.for_snapshot(&snapshot, None)
    ));
    assert!(!Arc::ptr_eq(
        &first,
        &IndexCache::default().for_snapshot(&snapshot, Some(key))
    ));
}

#[test]
fn out_of_order_fill_keeps_new_and_old_reader_arcs_separate() {
    let cache = IndexCache::default();
    let snapshot = snapshot();
    let old_key = identity(&snapshot);
    let mut new_key = old_key.clone();
    new_key.revision += 1;
    let old = Arc::new(RevisionIndexes::new(&snapshot.nodes));
    let new = Arc::new(RevisionIndexes::new(&snapshot.nodes));
    cache.publish(new_key.clone(), new.clone());
    cache.publish(old_key.clone(), old.clone());
    assert!(Arc::ptr_eq(
        &new,
        &cache.for_snapshot(&snapshot, Some(new_key))
    ));
    assert!(Arc::ptr_eq(
        &old,
        &cache.for_snapshot(&snapshot, Some(old_key))
    ));
}

#[test]
fn metadata_cache_does_not_serialize_or_admit_snapshot_payloads() {
    let cache = IndexCache::default();
    let mut snapshot = snapshot();
    snapshot.nodes[0].props = json!({"name":"x".repeat(17 * 1024 * 1024)});
    let key = identity(&snapshot);
    let first = cache.for_snapshot(&snapshot, Some(key.clone()));
    assert!(Arc::ptr_eq(
        &first,
        &cache.for_snapshot(&snapshot, Some(key))
    ));
}

#[test]
fn eviction_and_expiry_rebuild_without_invalidating_reader() {
    let cache = IndexCache::limits(Duration::from_millis(1), CACHE_BUDGET);
    let snapshot = snapshot();
    let key = identity(&snapshot);
    let old = cache.for_snapshot(&snapshot, Some(key.clone()));
    std::thread::sleep(Duration::from_millis(5));
    assert!(!Arc::ptr_eq(
        &old,
        &cache.for_snapshot(&snapshot, Some(key.clone()))
    ));
    cache.0.invalidate(&key);
    let next = cache.for_snapshot(&snapshot, Some(key));
    assert!(!Arc::ptr_eq(&old, &next));
    assert_eq!(old.page("page"), Some("page"));
}

#[test]
fn lazy_scope_index_is_reused_and_its_full_metadata_fits_admission_reservation() {
    let mut snapshot = snapshot();
    // Frame positions are additional metadata; roots/frames/notes must all fit.
    for i in 0..100 {
        snapshot.nodes.push(Node {
            id: format!("frame{i}"),
            parent_id: Some("page".into()),
            sort_key: i,
            kind: NodeKind::Frame,
            props: json!({"width":10,"height":10}),
        });
    }
    snapshot.notes = (0..5000)
        .map(|i| Note {
            id: format!("note{i}"),
            node_id: "page".into(),
            text: "Intent".into(),
            author: None,
            resolved: false,
            replies: vec![],
        })
        .collect();
    let snapshot = Arc::new(snapshot);
    let indexes = Arc::new(RevisionIndexes::new(&snapshot.nodes));
    let shared = SharedSnapshot::committed(snapshot.clone(), indexes.clone());
    shared.trusted_scoped().unwrap().unwrap();
    let first = indexes.read_scope.lock().unwrap().as_ref().unwrap().clone();
    let again = SharedSnapshot::committed(snapshot, indexes.clone());
    again.trusted_scoped().unwrap().unwrap();
    assert!(Arc::ptr_eq(
        &first,
        indexes.read_scope.lock().unwrap().as_ref().unwrap()
    ));
    assert!(first.estimated_bytes() <= indexes.read_weight);
    let scoped = again.scoped().unwrap();
    assert_eq!(scoped.roots().count(), 1);
    assert_eq!(scoped.frames().count(), 100);
    assert_eq!(scoped.children("page").count(), 100);
    assert_eq!(
        again.scoped().unwrap().notes_for_nodes(["page"]).len(),
        5000
    );
}

#[test]
fn cached_scope_index_for_another_snapshot_is_neither_used_nor_replaced() {
    let small = Arc::new(snapshot());
    let indexes = Arc::new(RevisionIndexes::new(&small.nodes));
    SharedSnapshot::committed(small.clone(), indexes.clone())
        .trusted_scoped()
        .unwrap()
        .unwrap();
    let first = indexes.read_scope.lock().unwrap().clone().unwrap();
    // Same revision/project, more nodes: the cached positions would be wrong.
    let mut grown = snapshot();
    for i in 0..3 {
        grown.nodes.insert(
            0,
            Node {
                id: format!("frame{i}"),
                parent_id: Some("page".into()),
                sort_key: i,
                kind: NodeKind::Frame,
                props: json!({"width":10,"height":10}),
            },
        );
    }
    let shared = SharedSnapshot::committed(Arc::new(grown), indexes.clone());
    let scoped = shared.trusted_scoped().unwrap().unwrap();
    assert_eq!(scoped.node("page").unwrap().id, "page");
    assert_eq!(scoped.children("page").count(), 3);
    assert_eq!(scoped.frames().count(), 3);
    let cached = indexes.read_scope.lock().unwrap().clone().unwrap();
    assert!(Arc::ptr_eq(&first, &cached));
    assert!(cached.indexes(&small));
}
