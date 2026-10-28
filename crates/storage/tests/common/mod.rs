#![allow(dead_code)]
use opencanvas_storage::*;
use serde_json::{json, Value};
use std::sync::Arc;

pub fn setup() -> (tempfile::TempDir, Store, Arc<Project>) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let project = store.create_project("fixture", "Fixture").unwrap();
    (temp, store, project)
}
pub fn create(id: &str, kind: NodeKind, parent: Option<&str>, props: Value) -> Operation {
    Operation::CreateNode {
        id: id.into(),
        kind,
        parent_id: parent.map(str::to_string),
        props,
        index: None,
    }
}
pub fn initial() -> Vec<Operation> {
    vec![
        create("page", NodeKind::Page, None, json!({"name":"Home"})),
        create(
            "frame",
            NodeKind::Frame,
            Some("page"),
            json!({"width":800,"height":600,"layout":{"mode":"column","padding":24,"gap":16}}),
        ),
        create(
            "text",
            NodeKind::Text,
            Some("frame"),
            json!({"text":"Tiếng Việt — OpenCanvas","width":"fill","height":"hug","fontId":"font_bundled","fontSize":32,"lineHeight":40}),
        ),
        create(
            "rect",
            NodeKind::Rectangle,
            Some("frame"),
            json!({"width":100,"height":40,"fill":"#1a7aff"}),
        ),
    ]
}
pub fn request(p: &Project, revision: u64, key: &str, operations: Vec<Operation>) -> BatchRequest {
    BatchRequest {
        project_id: p.id().into(),
        expected_revision: revision,
        idempotency_key: key.into(),
        operations,
    }
}
pub fn png() -> Vec<u8> {
    // Complete 1x1 RGBA PNG, not an arbitrary path or executable payload.
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ]
}
pub fn asset_request(p: &Project, revision: u64, key: &str) -> ImportAssetRequest {
    ImportAssetRequest {
        project_id: p.id().into(),
        expected_revision: revision,
        idempotency_key: key.into(),
        media_type: "image/png".into(),
        bytes: png(),
        font: None,
    }
}
