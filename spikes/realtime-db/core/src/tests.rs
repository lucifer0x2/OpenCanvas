use crate::*;
use serde_json::json;

fn request(operations: Vec<Value>) -> BatchRequest {
    BatchRequest {
        project_id: "test".into(),
        expected_revision: 0,
        idempotency_key: "key".into(),
        operations,
        dry_run: false,
    }
}
fn empty() -> State {
    State {
        project_id: "test".into(),
        revision: 0,
        nodes: vec![],
    }
}
fn page(id: &str) -> Value {
    json!({"op":"create_node","id":id,"parentId":null,"kind":"page","props":{}})
}
fn frame(id: &str, parent: &str) -> Value {
    json!({"op":"create_node","id":id,"parentId":parent,"kind":"frame","props":{"width":100}})
}

#[test]
fn full_candidate_allows_forward_parent_and_orders_changed_ids() {
    let input = empty();
    let (state, changed) = apply(&input, &request(vec![frame("z", "a"), page("a")])).unwrap();
    assert_eq!(changed, vec!["a", "z"]);
    assert_eq!(state.revision, 1);
    assert_eq!(input, empty());
}

#[test]
fn cycles_and_invalid_parenting_are_rejected_atomically() {
    let before = empty();
    for ops in [
        vec![page("p"), frame("a", "b"), frame("b", "a")],
        vec![frame("a", "missing")],
        vec![
            page("p"),
            frame("a", "p"),
            json!({"op":"reparent_node","id":"p","parentId":"a"}),
        ],
    ] {
        assert_eq!(
            apply(&before, &request(ops)).unwrap_err().code,
            "VALIDATION_FAILED"
        );
        assert!(before.nodes.is_empty());
    }
}

#[test]
fn strict_operation_and_property_validation() {
    for op in [
        json!({"op":"execute","code":"bad"}),
        json!({"op":"create_node","id":"p","kind":"page","props":{},"execute":"bad"}),
        json!({"op":"create_node","id":"p","kind":"page","props":{"onClick":"bad"}}),
        json!({"op":"create_node","id":"p","kind":"page","props":{"width":-1}}),
        json!({"op":"create_node","id":"p","kind":"page","props":{"text":123}}),
    ] {
        assert!(apply(&empty(), &request(vec![op])).is_err());
    }
}

#[test]
fn updates_shallow_merge_reparent_and_delete_entire_subtree() {
    let (state, _) = apply(
        &empty(),
        &request(vec![
            page("p"),
            frame("a", "p"),
            frame("b", "a"),
            frame("c", "p"),
        ]),
    )
    .unwrap();
    let mut req = request(vec![
        json!({"op":"update_node","id":"a","props":{"height":40}}),
        json!({"op":"reparent_node","id":"c","parentId":"a","sortKey":9}),
    ]);
    req.expected_revision = 1;
    let (state, _) = apply(&state, &req).unwrap();
    let node = state.nodes.iter().find(|n| n.id == "a").unwrap();
    assert_eq!(node.props, json!({"width":100,"height":40}));
    req.expected_revision = 2;
    req.operations = vec![json!({"op":"delete_subtree","id":"a"})];
    let (state, changed) = apply(&state, &req).unwrap();
    assert_eq!(changed, vec!["a", "b", "c"]);
    assert_eq!(state.nodes.len(), 1);
}

#[test]
fn canonical_hash_ignores_dry_run_and_object_key_order_but_keeps_operation_order() {
    let req = request(vec![page("p"), frame("f", "p")]);
    let hash = canonical_hash(&req).unwrap();
    let mut preview = req.clone();
    preview.dry_run = true;
    assert_eq!(hash, canonical_hash(&preview).unwrap());
    let reordered: BatchRequest=serde_json::from_str(r#"{"operations":[{"props":{},"kind":"page","parentId":null,"id":"p","op":"create_node"},{"props":{"width":100},"kind":"frame","parentId":"p","id":"f","op":"create_node"}],"idempotencyKey":"key","expectedRevision":0,"projectId":"test"}"#).unwrap();
    assert_eq!(hash, canonical_hash(&reordered).unwrap());
    preview.operations.reverse();
    assert_ne!(hash, canonical_hash(&preview).unwrap());
    preview = req.clone();
    preview.expected_revision = 1;
    assert_ne!(hash, canonical_hash(&preview).unwrap());
    preview = req.clone();
    preview.idempotency_key = "other".into();
    assert_ne!(hash, canonical_hash(&preview).unwrap());
}

#[test]
fn limits_and_revision_guard() {
    assert!(apply(&empty(), &request(vec![])).is_err());
    assert!(apply(&empty(), &request(vec![page("p"); 501])).is_err());
    let huge =
        json!({"op":"create_node","id":"p","kind":"page","props":{"name":"x".repeat(2*1024*1024)}});
    assert!(canonical_hash(&request(vec![huge])).is_err());
    let mut req = request(vec![page("p")]);
    req.expected_revision = 2;
    let error = apply(&empty(), &req).unwrap_err();
    assert_eq!(error.code, "REVISION_CONFLICT");
    assert_eq!(error.current_revision, Some(0));
    assert!(serde_json::from_value::<BatchRequest>(json!({"projectId":"p","expectedRevision":0,"idempotencyKey":"k","operations":[],"script":"bad"})).is_err());
}

#[test]
fn profiling_preserves_results_errors_and_input() {
    let before = empty();
    let valid = request(vec![page("p"), frame("f", "p")]);
    let mut profile = ApplyProfile::default();
    assert_eq!(
        apply_profiled(&before, &valid, &mut profile).unwrap(),
        apply(&before, &valid).unwrap()
    );
    let invalid = request(vec![frame("f", "missing")]);
    assert_eq!(
        apply_profiled(&before, &invalid, &mut profile)
            .unwrap_err()
            .code,
        apply(&before, &invalid).unwrap_err().code
    );
    assert_eq!(before, empty());
}
