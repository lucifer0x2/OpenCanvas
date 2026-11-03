use super::*;
use crate::Snapshot;
use serde_json::{json, Value};

const ZERO: &str = r#"{"id":"t","name":"T","type":"number","value":0.0}"#;
const NOTE: &str = r#"{"id":"n","nodeId":"a","text":"Old"}"#;

fn base() -> (Candidate, Stored) {
    let snapshot: Snapshot = serde_json::from_value(json!({
        "schemaVersion": crate::SCHEMA_VERSION, "projectId": "p", "slug": "p", "name": "P",
        "revision": 1,
        "nodes": [{"id":"a","parentId":null,"sortKey":0,"kind":"page","props":{"name":"A"}}],
        "tokens": [serde_json::from_str::<Value>(ZERO).unwrap()],
        "styles": [], "assets": [], "fonts": [],
        "notes": [serde_json::from_str::<Value>(NOTE).unwrap()],
        "retention": {"oldestCommitRevision": 1, "idempotencyPolicy": "kept"}
    }))
    .unwrap();
    let node = serde_json::to_string(&snapshot.nodes[0]).unwrap();
    let mut stored = Stored {
        nodes: vec![("a".into(), node)],
        tokens: vec![("t".into(), ZERO.into())],
        notes: vec![("n".into(), NOTE.into())],
        ..Stored::default()
    };
    stored.sort();
    (Candidate::from_snapshot(&snapshot), stored)
}

#[test]
fn unchanged_rows_write_nothing() {
    let (after, stored) = base();
    let rows = stored.delta(&after).unwrap();
    assert!(rows.ids.is_empty() && rows.removed_ids.is_empty());
}

#[test]
fn negative_zero_is_a_change_although_domain_equality_ignores_it() {
    let (mut after, stored) = base();
    after.tokens.get_mut("t").unwrap().value = json!(-0.0);
    let rows = stored.delta(&after).unwrap();
    assert_eq!(rows.kinds, ["token"]);
    assert_eq!(
        rows.payloads,
        [r#"{"id":"t","name":"T","type":"number","value":-0.0}"#]
    );
}

#[test]
fn inserts_and_removals_follow_kind_then_id_order() {
    let (mut after, stored) = base();
    let mut b = after.nodes["a"].clone();
    b.id = "b".into();
    after.nodes.insert("b".into(), b.clone());
    let mut zero = b;
    zero.id = "0".into();
    after.nodes.insert("0".into(), zero);
    after.nodes.remove("a");
    after.notes.remove("n");
    let rows = stored.delta(&after).unwrap();
    assert_eq!(rows.kinds, ["node", "node"]);
    assert_eq!(rows.ids, ["0", "b"]);
    assert_eq!(rows.removed_kinds, ["node", "note"]);
    assert_eq!(rows.removed_ids, ["a", "n"]);
}

#[test]
fn stored_text_that_reserialization_would_not_reproduce_is_rewritten_once() {
    let (after, mut stored) = base();
    stored.notes[0].1 = r#"{ "id": "n", "nodeId": "a", "text": "Old" }"#.into();
    let rows = stored.delta(&after).unwrap();
    assert_eq!(rows.ids, ["n"]);
    assert_eq!(rows.payloads, [NOTE]);
    assert!(rows.removed_ids.is_empty());
}
