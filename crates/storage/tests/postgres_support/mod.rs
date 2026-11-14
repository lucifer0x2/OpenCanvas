#![allow(dead_code)]
use opencanvas_storage::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

pub fn url() -> String {
    std::env::var("TASK19_POSTGRES_URL")
        .expect("run scripts/task19-check.sh: real fresh PG required")
}

pub fn config() -> PostgresConfig {
    let mut config = PostgresConfig::new(url());
    config.statement_timeout = Duration::from_secs(2);
    config.lock_timeout = Duration::from_millis(300);
    config
}

pub fn setup() -> (Arc<PostgresBackend>, Storage) {
    let backend = Arc::new(PostgresBackend::with_config(config()).unwrap());
    let storage = Storage::from_shared(backend.clone());
    (backend, storage)
}

pub fn cached() -> (PostgresBackend, SnapshotCache) {
    let backend = PostgresBackend::with_config(config()).unwrap();
    let cache = SnapshotCache::new(backend.cache_namespace());
    (backend.with_cache(cache.clone()).unwrap(), cache)
}

pub fn lock_entity_tables(client: &mut postgres::Client) -> postgres::Transaction<'_> {
    let mut lock = client.transaction().unwrap();
    lock.batch_execute(
        "LOCK TABLE opencanvas_runtime_v1.entities, opencanvas_runtime_v1.binaries IN ACCESS EXCLUSIVE MODE",
    )
    .unwrap();
    lock
}

pub fn project(storage: &Storage, label: &str) -> Arc<ProjectHandle> {
    storage
        .create_project(&format!("{label}-{}", uuid::Uuid::new_v4().simple()), label)
        .unwrap()
}

pub fn sql() -> postgres::Client {
    postgres::Client::connect(&url(), postgres::NoTls).unwrap()
}

pub fn request(id: &str, revision: u64, key: &str, operations: Vec<Operation>) -> BatchRequest {
    BatchRequest {
        project_id: id.into(),
        expected_revision: revision,
        idempotency_key: key.into(),
        operations,
    }
}

pub fn page(name: &str) -> Vec<Operation> {
    vec![Operation::CreateNode {
        id: "page".into(),
        kind: NodeKind::Page,
        parent_id: None,
        props: json!({"name": name}),
        index: None,
    }]
}

pub fn rename(name: &str) -> Vec<Operation> {
    vec![Operation::UpdateNode {
        merge: false,
        id: "page".into(),
        props: json!({"name": name}),
    }]
}

pub fn normalized(snapshot: Snapshot) -> serde_json::Value {
    let mut value = serde_json::to_value(snapshot).unwrap();
    for key in ["projectId", "slug", "name"] {
        value.as_object_mut().unwrap().remove(key);
    }
    value
}
