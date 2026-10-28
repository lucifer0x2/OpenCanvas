mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Map, Value};

fn probe(p: &Project, kind: &NodeKind, key: &str) -> Option<StorageError> {
    let mut props = Map::new();
    props.insert(key.into(), Value::Null);
    let parent = (*kind != NodeKind::Page).then_some("frame");
    let op = create("probe", kind.clone(), parent, Value::Object(props));
    p.apply_batch(request(p, 1, &format!("{}-{key}", kind.as_str()), vec![op]))
        .err()
}

/// The catalog is the validator's key table: every key it allows for a kind passes the
/// key check, every other key fails it with the catalog's own list for that kind.
#[test]
fn validator_accepts_exactly_the_catalog_keys_per_kind() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    for kind in &NODE_KINDS {
        let allowed = allowed_props(kind);
        for key in PROPS
            .iter()
            .map(|s| s.key)
            .chain(["cornerRadius", "zIndex"])
        {
            let field = format!("props.{key}");
            let error = probe(&p, kind, key);
            // A key error carries allowedKeys; a value error on the same field (for
            // example a null `animations` array) is not one.
            let key_error = error.as_ref().filter(|e| {
                e.field.as_deref() == Some(&field)
                    && e.node.as_ref().is_some_and(|n| n.allowed_keys.is_some())
            });
            if allowed.contains(&key) {
                assert!(key_error.is_none(), "{key} rejected on {kind:?}: {error:?}");
                continue;
            }
            let e = key_error.unwrap_or_else(|| panic!("{key} on {kind:?}: {error:?}"));
            assert_eq!(e.code, "VALIDATION_FAILED");
            assert_eq!(e.op_index, Some(0));
            let node = e.node.as_deref().unwrap();
            assert_eq!(node.node_id.as_deref(), Some("probe"));
            assert_eq!(node.node_kind.as_deref(), Some(kind.as_str()));
            let listed: Vec<&str> = node
                .allowed_keys
                .iter()
                .flatten()
                .map(String::as_str)
                .collect();
            assert_eq!(listed, allowed);
        }
    }
}

#[test]
fn unsupported_key_names_the_key_and_a_suggestion() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let op = create(
        "card",
        NodeKind::Rectangle,
        Some("frame"),
        json!({"width":10,"height":10,"cornerRadius":8}),
    );
    let e = p
        .apply_batch(request(&p, 1, "corner", vec![op]))
        .unwrap_err();
    assert_eq!(e.field.as_deref(), Some("props.cornerRadius"));
    assert_eq!(
        e.node.as_ref().unwrap().suggestion.as_deref(),
        Some("radius")
    );
    assert!(e
        .message
        .starts_with("Unsupported authored property 'cornerRadius'"));
    let wire = json!(e);
    for key in [
        "field",
        "nodeId",
        "nodeKind",
        "allowedKeys",
        "suggestion",
        "opIndex",
    ] {
        assert!(wire.get(key).is_some(), "{key} missing from {wire}");
    }

    let op = create(
        "box",
        NodeKind::Rectangle,
        Some("frame"),
        json!({"fontSize":12}),
    );
    let e = p.apply_batch(request(&p, 1, "font", vec![op])).unwrap_err();
    assert_eq!(e.field.as_deref(), Some("props.fontSize"));
    assert!(e.message.starts_with("Text properties require a text node"));
    assert!(e.node.as_ref().unwrap().suggestion.is_none());
}

#[test]
fn value_errors_keep_their_field_and_gain_node_context() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let op = create(
        "dim",
        NodeKind::Rectangle,
        Some("frame"),
        json!({"opacity":2}),
    );
    let e = p
        .apply_batch(request(&p, 1, "opacity", vec![op]))
        .unwrap_err();
    assert_eq!(e.field.as_deref(), Some("opacity"));
    assert_eq!(e.node.as_ref().unwrap().node_id.as_deref(), Some("dim"));
    assert!(e.node.as_ref().unwrap().allowed_keys.is_none());
}
