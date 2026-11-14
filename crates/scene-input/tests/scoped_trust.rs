//! Trusted scoped reads over rows that bypassed the current validator: every
//! malformed shape must surface as an error, never a panic or stack overflow.
#[allow(dead_code)]
mod incremental_support;
use incremental_support::*;
use opencanvas_scene_input::{read_scope::*, *};
use serde_json::{json, Value};
use std::sync::Arc;

fn trusted(snapshot: &Snapshot) -> ScopedSnapshot<'_> {
    ScopedSnapshot::from_validated(snapshot, Arc::new(ScopeIndex::new(snapshot)))
}

fn set(snapshot: &mut Snapshot, id: &str, props: Value) {
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == id)
        .unwrap()
        .props = props;
}

/// Expansion, resolution and frame preparation all fail with a validation error.
fn rejects(snapshot: &Snapshot, target: &str, field: &str) {
    let scoped = trusted(snapshot);
    let error = scoped
        .expand(ReadScope::Target(target), true)
        .and_then(|mut nodes| scoped.resolve(&mut nodes, "default"))
        .unwrap_err();
    assert_eq!(error.field.as_deref(), Some(field), "{error:?}");
    let options = SceneOptions {
        frame_id: "frame0".into(),
        width: 800.,
        height: 600.,
        theme: "default".into(),
        state: "default".into(),
    };
    assert!(scoped.prepare_input_with_scale(&options, 1.).is_err());
}

#[test]
fn malformed_instance_master_and_override_props_fail_without_panicking() {
    let mut snapshot = fixture(1, 0);
    set(&mut snapshot, "instance0", json!(["not", "an", "object"]));
    rejects(&snapshot, "frame0", "props.componentId");
    let mut snapshot = fixture(1, 0);
    set(&mut snapshot, "instance0", json!({"componentId":"missing"}));
    rejects(&snapshot, "frame0", "props.componentId");
    // A virtual child with non-object props cannot accept an override patch.
    let mut snapshot = fixture(1, 0);
    set(&mut snapshot, "label", json!("text"));
    rejects(&snapshot, "frame0", "props");
    // Non-object authored props reach resolution, which reports the full error.
    let mut snapshot = fixture(1, 0);
    set(&mut snapshot, "child0", json!(7));
    rejects(&snapshot, "frame0", "props");
}

#[test]
fn master_parent_cycle_and_deep_chains_are_bounded_errors() {
    let mut snapshot = fixture(1, 0);
    // `outer` (a component master) and `loop` parent each other: unreachable from
    // any page, but still a lookup dependency of the placed instance.
    snapshot.nodes.push(node(
        "loop",
        NodeKind::Frame,
        Some("outer"),
        json!({"width":1,"height":1}),
    ));
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "outer")
        .unwrap()
        .parent_id = Some("loop".into());
    rejects(&snapshot, "frame0", "parentId");

    let mut snapshot = fixture(1, 0);
    let mut parent = "frame0".to_string();
    for i in 0..MAX_NODES + 1 {
        let id = format!("deep{i}");
        snapshot
            .nodes
            .push(node(&id, NodeKind::Group, Some(&parent), json!({})));
        parent = id;
    }
    let scoped = trusted(&snapshot);
    let error = scoped
        .expand(ReadScope::Page("page0"), false)
        .err()
        .unwrap();
    assert_eq!(error.code, "INSTANCE_LIMIT");
    let leaf = scoped.expand(ReadScope::Target(&parent), false).err();
    assert_eq!(leaf.unwrap().code, "INSTANCE_LIMIT");
}

#[test]
fn an_index_bound_to_another_snapshot_is_rebuilt_never_dereferenced() {
    let small = fixture(1, 0);
    let large = fixture(4, 3);
    let stale = Arc::new(ScopeIndex::new(&small));
    assert!(stale.indexes(&small));
    assert!(!stale.indexes(&large));
    let mut renumbered = small.clone();
    renumbered.revision += 1;
    assert!(!stale.indexes(&renumbered));
    let mut reordered = small.clone();
    reordered.nodes.reverse();
    assert!(!stale.indexes(&reordered));
    let reference = ScopedSnapshot::validate(&large).unwrap();
    let scoped = ScopedSnapshot::from_validated(&large, stale);
    for target in ["frame3", "instance2", "page3"] {
        let mut expected = reference.expand(ReadScope::Target(target), true).unwrap();
        let mut actual = scoped.expand(ReadScope::Target(target), true).unwrap();
        assert_eq!(
            scoped.resolve(&mut actual, "night").unwrap(),
            reference.resolve(&mut expected, "night").unwrap()
        );
    }
    assert_eq!(scoped.roots().count(), reference.roots().count());
}
