mod common;
use common::*;
use opencanvas_storage::*;
use rusqlite::Connection;

fn downgrade(path: &std::path::Path, failure: bool) {
    let c = Connection::open(path).unwrap();
    c.execute_batch("DROP TABLE retention_metadata; UPDATE project_meta SET schema_version=1; PRAGMA user_version=1;").unwrap();
    if failure {
        // The migration creates its table, then this trigger aborts its metadata
        // update. Rollback must undo the earlier DDL too.
        c.execute_batch("CREATE TRIGGER reject_upgrade BEFORE UPDATE OF schema_version ON project_meta BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    }
}

#[test]
fn v1_upgrade_preserves_authored_revision_and_replay() {
    let (temp, store, p) = setup();
    let req = request(&p, 0, "initial", initial());
    p.apply_batch(req.clone()).unwrap();
    let before = p.snapshot().unwrap();
    drop(p);
    drop(store);
    let db = temp.path().join("fixture/db.sql");
    downgrade(&db, false);
    let store = Store::open(temp.path()).unwrap();
    let p = store.open_project("fixture").unwrap();
    let after = p.snapshot().unwrap();
    assert_eq!(after.schema_version, 2);
    assert_eq!(after.project_id, before.project_id);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.nodes, before.nodes);
    assert_eq!(after.fonts, before.fonts);
    assert!(p.apply_batch(req).unwrap().replayed);
    assert_eq!(p.commits_after(0, 10).unwrap().len(), 1);
    assert_eq!(after.retention.oldest_commit_revision, 1);
}

#[test]
fn failed_upgrade_rolls_back_ddl_metadata_and_authored_data() {
    let (temp, store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let before = p.snapshot().unwrap();
    drop(p);
    drop(store);
    let db = temp.path().join("fixture/db.sql");
    downgrade(&db, true);
    let store = Store::open(temp.path()).unwrap();
    assert_eq!(
        store.open_project("fixture").err().unwrap().code,
        "MIGRATION_FAILED"
    );
    let c = Connection::open(&db).unwrap();
    assert_eq!(
        c.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        c.query_row("SELECT schema_version FROM project_meta", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert!(!c
        .prepare("SELECT 1 FROM sqlite_schema WHERE name='retention_metadata'")
        .unwrap()
        .exists([])
        .unwrap());
    assert_eq!(
        c.query_row("SELECT current_revision FROM project_meta", [], |r| r
            .get::<_, i64>(0))
            .unwrap() as u64,
        before.revision
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM nodes", [], |r| r.get::<_, i64>(0))
            .unwrap() as usize,
        before.nodes.len()
    );
    c.execute_batch("DROP TRIGGER reject_upgrade").unwrap();
    drop(c);
    assert_eq!(
        store
            .open_project("fixture")
            .unwrap()
            .snapshot()
            .unwrap()
            .nodes,
        before.nodes
    );
}

#[test]
fn mismatched_or_future_schema_is_never_migrated() {
    for (pragma, meta) in [(1, 2), (99, 99)] {
        let (temp, store, p) = setup();
        drop(p);
        let c = Connection::open(temp.path().join("fixture/db.sql")).unwrap();
        c.execute_batch(&format!(
            "PRAGMA user_version={pragma}; UPDATE project_meta SET schema_version={meta}"
        ))
        .unwrap();
        assert_eq!(
            store.open_project("fixture").err().unwrap().code,
            "SCHEMA_UNSUPPORTED"
        );
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            pragma
        );
    }
}
