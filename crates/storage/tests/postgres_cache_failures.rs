mod postgres_support;
use opencanvas_storage::*;
use postgres_support::*;
use std::{process::Command, time::Duration};

struct PausedPostgres(String);

impl PausedPostgres {
    fn pause() -> Self {
        let name = std::env::var("TASK19_POSTGRES_CONTAINER").expect("owned container required");
        assert!(name.starts_with("opencanvas-task19.") && name.ends_with("-pg"));
        assert!(Command::new("docker")
            .args(["pause", &name])
            .output()
            .unwrap()
            .status
            .success());
        Self(name)
    }
}

impl Drop for PausedPostgres {
    fn drop(&mut self) {
        let _ = Command::new("docker").args(["unpause", &self.0]).output();
    }
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn warm_cache_cannot_mask_a_paused_postgres_authority() {
    let mut pg_config = config();
    pg_config.operation_timeout = Duration::from_secs(2);
    pg_config.connect_timeout = Duration::from_millis(400);
    let backend = PostgresBackend::with_config(pg_config).unwrap();
    let cache = SnapshotCache::new(backend.cache_namespace());
    let storage = Storage::new(backend.with_cache(cache).unwrap());
    let p = project(&storage, "pg-down");
    p.apply_batch(request(p.id(), 0, "seed", page("Warm")))
        .unwrap();
    let expected = p.snapshot().unwrap();
    let mut control = sql();
    let lock = lock_entity_tables(&mut control);
    assert_eq!(p.snapshot().unwrap(), expected);
    lock.rollback().unwrap();
    let paused = PausedPostgres::pause();
    let error = p.snapshot().unwrap_err();
    assert_eq!(error.code, "STORAGE_UNAVAILABLE");
    assert!(error.retryable);
    drop(paused);
    assert_eq!(p.snapshot().unwrap(), expected);
}
