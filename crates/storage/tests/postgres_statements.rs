mod postgres_support;
use opencanvas_storage::*;
use postgres_support::*;

const ALTER: &str = "ALTER TABLE opencanvas_runtime_v1.projects";

fn revision(control: &mut postgres::Client, id: &str) -> i64 {
    control
        .query_one(
            "SELECT revision FROM opencanvas_runtime_v1.projects WHERE project_id=$1",
            &[&id],
        )
        .unwrap()
        .get(0)
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn invalidated_cached_statements_fail_once_then_reprepare_without_retrying_writes() {
    let mut pg_config = config();
    pg_config.pool_size = 1;
    let backend = PostgresBackend::with_config(pg_config).unwrap();
    let cache = SnapshotCache::new(backend.cache_namespace());
    let backend = backend.with_cache(cache).unwrap();
    let id = backend
        .create_project(&format!("stmt-{}", uuid::Uuid::new_v4().simple()), "stmt")
        .unwrap()
        .project_id;
    backend
        .apply_batch(&id, request(&id, 0, "seed", page("Seed")))
        .unwrap();
    let warm = backend.snapshot(&id).unwrap();
    let mut control = sql();

    // A changed `projects.*` row type invalidates the worker's cached lookup plan.
    control
        .batch_execute(&format!("{ALTER} ADD COLUMN statement_probe INTEGER"))
        .unwrap();
    let rename_request = request(&id, 1, "rename", rename("Renamed"));
    let error = backend
        .apply_batch(&id, rename_request.clone())
        .unwrap_err();
    assert_eq!(error.code, "STORAGE_FAILED", "{error}");
    assert_eq!(
        revision(&mut control, &id),
        1,
        "the failed write is not retried"
    );
    let committed = backend.apply_batch(&id, rename_request).unwrap();
    assert_eq!(committed.committed_revision, 2);
    assert!(!committed.replayed);
    let renamed = backend.snapshot(&id).unwrap();
    assert_eq!(renamed.revision, 2);
    assert_ne!(renamed, warm);

    // A warm cache hit is never served when the probe itself fails.
    assert_eq!(backend.snapshot(&id).unwrap(), renamed);
    control
        .batch_execute(&format!("{ALTER} DROP COLUMN statement_probe"))
        .unwrap();
    assert_eq!(backend.snapshot(&id).unwrap_err().code, "STORAGE_FAILED");
    assert_eq!(backend.snapshot(&id).unwrap(), renamed);
    assert_eq!(backend.commits_after(&id, 0, 10).unwrap().len(), 2);
}
