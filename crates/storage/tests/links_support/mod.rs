#![allow(dead_code)]
use crate::common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

pub fn frame(id: &str, page: &str, name: &str) -> Operation {
    create(
        id,
        NodeKind::Frame,
        Some(page),
        json!({"name":name,"width":390,"height":844}),
    )
}

pub fn button(id: &str, parent: &str, links: Value) -> Operation {
    create(
        id,
        NodeKind::Rectangle,
        Some(parent),
        json!({"width":80,"height":40,"links":links}),
    )
}

pub fn update(id: &str, props: Value) -> Operation {
    Operation::UpdateNode {
        id: id.into(),
        props,
    }
}

pub fn delete(id: &str) -> Operation {
    Operation::DeleteSubtree { id: id.into() }
}

/// Two pages: frames a, b, d on page 1, frame c on page 2; button `a-btn` in a, `c-btn` in c.
pub fn screens(p: &Project) -> u64 {
    let ops = vec![
        create("p1", NodeKind::Page, None, json!({"name":"One"})),
        create("p2", NodeKind::Page, None, json!({"name":"Two"})),
        frame("a", "p1", "Home"),
        frame("b", "p1", "Checkout"),
        frame("d", "p1", "Menu"),
        frame("c", "p2", "Done"),
        create(
            "a-row",
            NodeKind::Frame,
            Some("a"),
            json!({"width":390,"height":80}),
        ),
        button("a-btn", "a-row", json!([])),
        button("c-btn", "c", json!([])),
    ];
    p.apply_batch(request(p, 0, "screens", ops)).unwrap();
    1
}

pub fn props(p: &Project, id: &str) -> Value {
    p.snapshot()
        .unwrap()
        .nodes
        .into_iter()
        .find(|n| n.id == id)
        .unwrap()
        .props
}

/// Applies one batch that must fail validation; the revision must not move.
pub fn rejected(p: &Project, revision: u64, key: &str, ops: Vec<Operation>) -> StorageError {
    let e = p.apply_batch(request(p, revision, key, ops)).unwrap_err();
    assert_eq!(e.code, "VALIDATION_FAILED", "{e:?}");
    assert_eq!(p.snapshot().unwrap().revision, revision);
    e
}

pub fn node_id(e: &StorageError) -> Option<&str> {
    e.node.as_ref().and_then(|n| n.node_id.as_deref())
}

pub fn suggestion(e: &StorageError) -> Option<&str> {
    e.node.as_ref().and_then(|n| n.suggestion.as_deref())
}
