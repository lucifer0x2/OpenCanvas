mod postgres_support;
use opencanvas_storage::*;
use postgres_support::*;

struct RestoreNamespace(String);
impl Drop for RestoreNamespace {
    fn drop(&mut self) {
        let _ = sql().execute(
            "UPDATE opencanvas_runtime_v1.schema_registry SET namespace=$1 WHERE singleton",
            &[&self.0],
        );
    }
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn wrong_cache_namespace_is_explicitly_rejected_before_runtime_injection() {
    let backend = PostgresBackend::with_config(config()).unwrap();
    let cache = SnapshotCache::new("unrelated-database");
    let error = backend
        .with_cache(cache)
        .err()
        .expect("namespace mismatch must fail");
    assert_eq!(error.code, "INVALID_CACHE_CONFIG");
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn authoritative_namespace_rotation_invalidates_a_warm_cache_until_reconnected() {
    let (backend, cache) = cached();
    let _restore = RestoreNamespace(backend.cache_namespace().to_owned());
    let storage = Storage::new(backend);
    let p = project(&storage, "namespace");
    p.apply_batch(request(p.id(), 0, "seed", page("Warm")))
        .unwrap();
    let expected = p.snapshot().unwrap();
    let replacement = uuid::Uuid::new_v4().to_string();
    sql()
        .execute(
            "UPDATE opencanvas_runtime_v1.schema_registry SET namespace=$1 WHERE singleton",
            &[&replacement],
        )
        .unwrap();
    let mut control = sql();
    let lock = lock_entity_tables(&mut control);
    assert_eq!(p.snapshot().unwrap_err().code, "STORAGE_BUSY");
    lock.rollback().unwrap();
    assert_eq!(p.snapshot().unwrap(), expected);
    let reconnected = PostgresBackend::with_config(config()).unwrap();
    assert_eq!(reconnected.cache_namespace(), replacement);
    assert_eq!(
        reconnected.clone().with_cache(cache).err().unwrap().code,
        "INVALID_CACHE_CONFIG"
    );
    let new_cache = SnapshotCache::new(reconnected.cache_namespace());
    let new_storage = Storage::new(reconnected.with_cache(new_cache).unwrap());
    assert_eq!(new_storage.snapshot(p.id()).unwrap(), expected);
    let lock = lock_entity_tables(&mut control);
    assert_eq!(new_storage.snapshot(p.id()).unwrap(), expected);
    lock.rollback().unwrap();
}
