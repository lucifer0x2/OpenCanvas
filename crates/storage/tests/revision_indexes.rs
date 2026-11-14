mod postgres_support;
mod scoped_support;
use opencanvas_storage::*;
use scoped_support::*;
use serde_json::json;
use std::sync::Arc;

fn exercise(storage: &Storage, p: &ProjectHandle) {
    seed(p);
    let old = storage.shared_snapshot(p.id()).unwrap();
    let again = storage.shared_snapshot(p.id()).unwrap();
    assert!(Arc::ptr_eq(&old.indexes, &again.indexes));
    assert_eq!(old.indexes.page("instance"), Some("p1"));
    assert_eq!(
        old.indexes.component_users("master").collect::<Vec<_>>(),
        ["instance"]
    );
    assert_eq!(
        old.indexes
            .reference_users("style:fill")
            .collect::<Vec<_>>(),
        ["r1"]
    );
    let context = old.edit_context(vec!["p1".into()]).unwrap();
    assert_eq!(
        context,
        edit_context((*old.snapshot).clone(), vec!["p1".into()]).unwrap()
    );
    assert_eq!(context, again.edit_context(vec!["p1".into()]).unwrap());

    let base = old.snapshot.revision;
    let preview = BatchRequest {
        project_id: p.id().into(),
        expected_revision: base,
        idempotency_key: "preview-index".into(),
        operations: vec![update("instance", json!({"componentId":"leaf"}))],
    };
    p.dry_run_batch(preview.clone()).unwrap();
    let mut failed = preview.clone();
    failed.idempotency_key = "bad-index".into();
    failed
        .operations
        .push(update("missing", json!({"width":1})));
    assert!(p.apply_batch(failed).is_err());
    assert!(Arc::ptr_eq(
        &old.indexes,
        &storage.shared_snapshot(p.id()).unwrap().indexes
    ));

    let committed = BatchRequest {
        idempotency_key: "commit-index".into(),
        operations: vec![update("r1", json!({"fill":"#abcdef"}))],
        ..preview.clone()
    };
    p.apply_batch(committed.clone()).unwrap();
    let next = storage.shared_snapshot(p.id()).unwrap();
    assert_eq!(next.snapshot.revision, base + 1);
    assert!(!Arc::ptr_eq(&old.indexes, &next.indexes));
    assert!(next.indexes.reference_users("style:fill").next().is_none());
    assert_eq!(
        next.indexes.component_users("master").collect::<Vec<_>>(),
        ["instance"]
    );
    assert_eq!(
        old.indexes
            .reference_users("style:fill")
            .collect::<Vec<_>>(),
        ["r1"]
    );
    assert!(p.apply_batch(committed).unwrap().replayed);
    assert!(Arc::ptr_eq(
        &next.indexes,
        &storage.shared_snapshot(p.id()).unwrap().indexes
    ));

    strict(
        p,
        "new-user",
        vec![update("instance", json!({"componentId":"leaf"}))],
    );
    let moved = storage.shared_snapshot(p.id()).unwrap();
    assert!(moved.indexes.component_users("master").next().is_none());
    assert_eq!(
        moved.indexes.component_users("leaf").collect::<Vec<_>>(),
        ["instance", "nested"]
    );
    let old_context = old.edit_context(vec!["p1".into()]).unwrap();
    assert_eq!(old_context["expectedRevision"], base);
    assert_eq!(
        old.snapshot
            .nodes
            .iter()
            .find(|n| n.id == "instance")
            .unwrap()
            .props["componentId"],
        "master"
    );

    // A resource write gets a fresh identity even if topology is unchanged.
    strict(p, "resource-index", vec![token("base", json!("#445566"))]);
    let resources = storage.shared_snapshot(p.id()).unwrap();
    assert_eq!(resources.snapshot.revision, base + 3);
    assert!(!Arc::ptr_eq(&resources.indexes, &moved.indexes));
    assert_eq!(
        resources.edit_context(vec!["p1".into()]).unwrap(),
        edit_context((*resources.snapshot).clone(), vec!["p1".into()]).unwrap()
    );
}

#[test]
fn sqlite_commits_reads_previews_replays_and_old_readers_share_revision_indexes() {
    let (_temp, storage, p) = sqlite("revision-index");
    exercise(&storage, &p);
}

#[test]
#[ignore = "fresh temporary PostgreSQL via scripts/task19-check.sh"]
fn postgres_commits_reads_previews_replays_and_old_readers_share_revision_indexes() {
    let (backend, _) = postgres_support::cached();
    let storage = Storage::new(backend);
    let p = postgres_support::project(&storage, "revision-index");
    exercise(&storage, &p);
}

#[test]
#[ignore = "fresh temporary PostgreSQL via scripts/task19-check.sh"]
fn postgres_generation_namespace_and_schema_are_authoritative() {
    use postgres_support::*;
    let (backend, _) = cached();
    let namespace = backend.cache_namespace().to_owned();
    struct Restore(String);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = sql().execute(
                "UPDATE opencanvas_runtime_v1.schema_registry SET namespace=$1 WHERE singleton",
                &[&self.0],
            );
        }
    }
    let _restore = Restore(namespace);
    let storage = Storage::new(backend);
    let p = project(&storage, "recreated-index");
    let old = storage.shared_snapshot(p.id()).unwrap();
    sql()
        .execute(
            "DELETE FROM opencanvas_runtime_v1.projects WHERE project_id=$1",
            &[&p.id()],
        )
        .unwrap();
    assert!(storage.shared_snapshot(p.id()).is_err());
    sql().execute("INSERT INTO opencanvas_runtime_v1.projects(project_id,slug,name,schema_version,revision) VALUES($1,$2,$3,$4,0)", &[&p.id(), &old.snapshot.slug, &b"Replacement".to_vec(), &(SCHEMA_VERSION as i32)]).unwrap();
    let recreated = storage.shared_snapshot(p.id()).unwrap();
    assert_eq!(old.snapshot.revision, recreated.snapshot.revision);
    assert!(!Arc::ptr_eq(&old.indexes, &recreated.indexes));
    let replacement = uuid::Uuid::new_v4().to_string();
    sql()
        .execute(
            "UPDATE opencanvas_runtime_v1.schema_registry SET namespace=$1 WHERE singleton",
            &[&replacement],
        )
        .unwrap();
    let rotated = storage.shared_snapshot(p.id()).unwrap();
    assert!(!Arc::ptr_eq(&recreated.indexes, &rotated.indexes));
    sql().execute("UPDATE opencanvas_runtime_v1.projects SET schema_version=schema_version+1 WHERE project_id=$1", &[&p.id()]).unwrap();
    assert!(
        matches!(storage.shared_snapshot(p.id()), Err(error) if error.code=="SCHEMA_UNSUPPORTED")
    );
}
