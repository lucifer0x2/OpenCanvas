#![allow(dead_code)]
use opencanvas_storage::*;
use serde_json::json;
use std::sync::Arc;

pub fn setup() -> (tempfile::TempDir, Arc<dyn StorageBackend>, Storage) {
    let root = tempfile::tempdir().unwrap();
    let backend: Arc<dyn StorageBackend> =
        Arc::new(SqliteBackend::new(Store::open(root.path()).unwrap()));
    let storage = Storage::from_shared(backend.clone());
    (root, backend, storage)
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
        props: json!({"name":name}),
        index: None,
    }]
}

pub fn rename(name: &str) -> Vec<Operation> {
    vec![Operation::UpdateNode {
        id: "page".into(),
        props: json!({"name":name}),
    }]
}

pub fn file(id: &str, revision: u64, key: &str, path: &str) -> BatchFileRequest {
    BatchFileRequest {
        project_id: id.into(),
        expected_revision: revision,
        idempotency_key: key.into(),
        path: path.into(),
    }
}
