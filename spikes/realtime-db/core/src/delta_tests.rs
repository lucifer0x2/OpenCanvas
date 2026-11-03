use crate::{apply, apply_delta, scalar_update_ids, ApplyProfile, BatchRequest, Node, State};
use serde_json::{json, Value};

fn fixture() -> State {
    State {
        project_id: "delta".into(),
        revision: 10,
        nodes: vec![
            Node {
                id: "frame".into(),
                parent_id: Some("page".into()),
                kind: "frame".into(),
                sort_key: 0,
                props: json!({"fill":"red","width":40}),
            },
            Node {
                id: "page".into(),
                parent_id: None,
                kind: "page".into(),
                sort_key: 0,
                props: json!({"name":"Page"}),
            },
            Node {
                id: "text".into(),
                parent_id: Some("frame".into()),
                kind: "text".into(),
                sort_key: 2,
                props: json!({"text":"old","fontSize":12}),
            },
        ],
    }
}

fn patch(id: &str, props: Value) -> Value {
    json!({"op":"update_node","id":id,"props":props})
}

fn differential(operations: Vec<Value>) {
    let before = fixture();
    let request = BatchRequest {
        project_id: before.project_id.clone(),
        expected_revision: before.revision,
        idempotency_key: "key".into(),
        operations,
        dry_run: false,
    };
    let ids = scalar_update_ids(&request).unwrap();
    let selected = State {
        nodes: before
            .nodes
            .iter()
            .filter(|node| ids.contains(&node.id))
            .cloned()
            .collect(),
        ..before.clone()
    };
    let mut profile = ApplyProfile::default();
    let delta = apply_delta(&selected, &request, Some(&mut profile));
    match (apply(&before, &request), delta) {
        (Ok((full, changed)), Ok((delta, delta_changed))) => {
            let mut merged = before.clone();
            for node in delta.nodes {
                let target = merged
                    .nodes
                    .iter_mut()
                    .find(|old| old.id == node.id)
                    .unwrap();
                *target = node;
            }
            merged.revision = delta.revision;
            assert_eq!(full, merged);
            assert_eq!(changed, delta_changed);
            assert_eq!(profile.sort_us, 0, "delta has no full candidate sorting");
        }
        (Err(full), Err(delta)) => assert_eq!(
            serde_json::to_value(full).unwrap(),
            serde_json::to_value(delta).unwrap()
        ),
        other => panic!("full/delta result mismatch: {other:?}"),
    }
}

#[test]
fn every_spike_scalar_matches_full_validation_at_final_candidate() {
    differential(vec![
        patch(
            "text",
            json!({"name":"renamed","text":"new","fill":"#fff","x":-1,"y":2,"width":3,"height":4,"fontSize":20,"opacity":0.3}),
        ),
        patch("frame", json!({"fill":"blue","width":99})),
    ]);
    for (key, good, bad) in [
        ("name", json!("name"), json!(true)),
        ("text", json!("text"), json!(4)),
        ("fill", json!("paint"), json!({"ref":"token"})),
        ("x", json!(-100000), json!(-100001)),
        ("y", json!(100000), json!(100001)),
        ("width", json!(0), json!(-1)),
        ("height", json!(100000), json!(100001)),
        ("fontSize", json!(16), json!("16")),
        ("opacity", json!(1), json!(1.01)),
    ] {
        differential(vec![patch("text", json!({key:bad.clone()}))]);
        differential(vec![
            patch("text", json!({key:bad})),
            patch("text", json!({key:good})),
        ]);
    }
    differential(vec![patch("page", json!({"text":"wrong kind"}))]);
    differential(vec![patch("frame", json!({"unrecognized":3}))]);
    differential(vec![patch("text", json!({"name":"x".repeat(100001)}))]);
}

#[test]
fn repeated_ids_missing_nodes_and_invalid_objects_preserve_ordered_errors() {
    differential(vec![
        patch("text", json!({"x":1})),
        patch("text", json!({"x":2})),
        patch("frame", json!({"height":50})),
    ]);
    differential(vec![
        patch("text", json!({"opacity":2})),
        patch("missing", json!({"opacity":0.5})),
    ]);
    differential(vec![
        patch("missing", json!({})),
        patch("text", json!(null)),
    ]);
    differential(vec![
        patch("text", json!(null)),
        patch("missing", json!({})),
    ]);
}

#[test]
fn classifier_falls_back_without_rejecting_structural_or_malformed_operations() {
    for op in [
        json!({"op":"delete_subtree","id":"text"}),
        json!({"op":"update_node","id":"text","props":{},"unknown":true}),
        patch("missing\0", json!({})),
        json!({"op":"unknown"}),
    ] {
        let request = BatchRequest {
            project_id: "delta".into(),
            expected_revision: 10,
            idempotency_key: "key".into(),
            operations: vec![patch("missing", json!({})), op],
            dry_run: false,
        };
        assert!(scalar_update_ids(&request).is_none());
        assert_eq!(
            apply(&fixture(), &request).unwrap_err().message,
            "Node does not exist"
        );
    }
}
