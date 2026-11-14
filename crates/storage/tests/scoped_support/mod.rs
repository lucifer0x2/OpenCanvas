#![allow(dead_code)]
use opencanvas_storage::*;
use serde_json::{json, Value};
use std::sync::Arc;
pub mod dependencies;
pub mod notes;
pub mod pages;
pub mod roots;
pub mod semantics;

pub fn seed(p: &ProjectHandle) {
    strict(p, "seed", serde_json::from_value(json!([
        {"op":"upsert_token","token":{"id":"base","name":"Base","type":"color","value":"#112233"}},
        {"op":"upsert_token","token":{"id":"alias","name":"Alias","type":"color","value":{"default":{"aliasId":"base"},"themes":{"night":{"aliasId":"base"}}}}},
        {"op":"upsert_style","style":{"id":"fill","name":"Fill","kind":"fill","value":{"tokenId":"alias"}}},
        {"op":"create_node","id":"p1","kind":"page","parentId":null,"props":{"name":"One"}},
        {"op":"create_node","id":"p2","kind":"page","parentId":null,"props":{"name":"Two"}},
        {"op":"create_node","id":"library","kind":"page","parentId":null,"props":{"name":"Library"}},
        {"op":"create_node","id":"f1","kind":"frame","parentId":"p1","props":{"width":400,"height":400}},
        {"op":"create_node","id":"f2","kind":"frame","parentId":"p2","props":{"width":400,"height":400}},
        {"op":"create_node","id":"r1","kind":"rectangle","parentId":"f1","props":{"width":20,"height":20,"fill":{"styleId":"fill"}}},
        {"op":"create_node","id":"r2","kind":"rectangle","parentId":"f2","props":{"width":20,"height":20}},
        {"op":"create_node","id":"leaf","kind":"frame","parentId":"library","props":{"component":{"name":"Leaf"},"width":40,"height":40}},
        {"op":"create_node","id":"leaf_box","kind":"rectangle","parentId":"leaf","props":{"width":10,"height":10}},
        {"op":"create_node","id":"master","kind":"frame","parentId":"library","props":{"component":{"name":"Master"},"width":100,"height":100}},
        {"op":"create_node","id":"nested","kind":"instance","parentId":"master","props":{"componentId":"leaf"}},
        {"op":"create_node","id":"instance","kind":"instance","parentId":"f1","props":{"componentId":"master"}}
    ])).unwrap());
}
pub fn update(id: &str, value: Value) -> Operation {
    Operation::UpdateNode {
        merge: false,
        id: id.into(),
        props: value,
    }
}
pub fn strict(p: &ProjectHandle, key: &str, operations: Vec<Operation>) -> BatchResult {
    p.apply_batch(BatchRequest {
        project_id: p.id().into(),
        expected_revision: p.snapshot().unwrap().revision,
        idempotency_key: key.into(),
        operations,
    })
    .unwrap()
}
pub fn scoped(
    p: &ProjectHandle,
    revision: u64,
    key: &str,
    pages: &[&str],
    operations: Vec<Operation>,
) -> ScopedBatchRequest {
    ScopedBatchRequest {
        project_id: p.id().into(),
        expected_revision: revision,
        idempotency_key: key.into(),
        page_ids: pages.iter().map(|s| (*s).into()).collect(),
        operations,
    }
}
pub fn sqlite(label: &str) -> (tempfile::TempDir, Storage, Arc<ProjectHandle>) {
    let temp = tempfile::tempdir().unwrap();
    let storage: Storage = Store::open(temp.path()).unwrap().into();
    let p = storage.create_project(label, label).unwrap();
    (temp, storage, p)
}
pub fn token(id: &str, value: Value) -> Operation {
    Operation::UpsertToken {
        token: Token {
            id: id.into(),
            name: id.into(),
            token_type: "color".into(),
            value,
        },
    }
}
pub fn font(p: &ProjectHandle, key: &str, family: &str, changed_bytes: bool) {
    let mut bytes = include_bytes!("../../../../renderer/fonts/NotoSans.ttf").to_vec();
    if changed_bytes {
        bytes.push(0);
    }
    p.import_asset(ImportAssetRequest {
        project_id: p.id().into(),
        expected_revision: p.snapshot().unwrap().revision,
        idempotency_key: key.into(),
        media_type: "font/ttf".into(),
        bytes,
        font: Some(FontRegistration {
            id: "uploaded".into(),
            family: family.into(),
            weight: 400,
            style: "normal".into(),
            license: Some(key.into()),
        }),
    })
    .unwrap();
}
pub fn ops(value: Value) -> Vec<Operation> {
    serde_json::from_value(value).unwrap()
}
pub fn code(p: &ProjectHandle, request: ScopedBatchRequest) -> String {
    p.apply_scoped_batch(request).unwrap_err().code
}
pub fn revision(p: &ProjectHandle) -> u64 {
    p.snapshot().unwrap().revision
}
