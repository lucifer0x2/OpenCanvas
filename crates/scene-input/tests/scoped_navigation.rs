#[allow(dead_code)]
mod incremental_support;
use incremental_support::*;
use opencanvas_scene_input::{read_scope::*, *};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn expanded_navigation_matches_full_incoming_sources_and_source_order() {
    let mut snapshot = fixture(4, 2);
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "child3")
        .unwrap()
        .props["links"] = json!([{"to":"frame0"}]);
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "child0")
        .unwrap()
        .props["links"] = json!([{"to":"frame2"}]);
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "instance3")
        .unwrap()
        .props["overrides"] = json!({"nested":{"fill":"#123456"}});
    snapshot.nodes.reverse();
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    let nodes = scoped.expand(ReadScope::Target("frame0"), false).unwrap();
    let metadata = scoped
        .navigation_expanded_nodes(&nodes.expanded, "frame0")
        .unwrap();
    let full = instances::expand(&snapshot.nodes).unwrap();
    let by_id: BTreeMap<_, _> = full.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut ids = BTreeSet::from(["frame2"]);
    ids.extend(
        full.iter()
            .filter(|n| {
                node_links(&n.props)
                    .iter()
                    .any(|link| link.to.as_deref() == Some("frame0"))
            })
            .map(|n| n.id.as_str()),
    );
    let mut pending: Vec<_> = ids.iter().copied().collect();
    while let Some(id) = pending.pop() {
        if let Some(parent) = by_id[id].parent_id.as_deref() {
            if ids.insert(parent) {
                pending.push(parent);
            }
        }
    }
    let expected: Vec<_> = full
        .iter()
        .filter(|n| ids.contains(n.id.as_str()))
        .cloned()
        .collect();
    assert_eq!(metadata, expected);
    assert!(!metadata.iter().any(|n| n.id == "extra3_0"));
}

#[test]
fn component_navigation_and_instance_navigation_overrides_never_gain_trusted_capability() {
    for (id, props) in [
        ("label", json!({"links":[{"to":"frame0"}]})),
        ("outer", json!({"links":[{"to":"frame0"}]})),
        ("instance0", json!({"links":[{"to":"frame0"}]})),
        (
            "instance0",
            json!({"overrides":{"nested":{"links":[{"to":"frame0"}]}}}),
        ),
    ] {
        let mut snapshot = fixture(1, 0);
        snapshot
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .unwrap()
            .props
            .as_object_mut()
            .unwrap()
            .extend(props.as_object().unwrap().clone());
        let full = resolved_expanded_nodes(&snapshot, "default").unwrap_err();
        let scope = ScopedSnapshot::validate(&snapshot).err().unwrap();
        assert_eq!(
            serde_json::to_value(scope).unwrap(),
            serde_json::to_value(full).unwrap()
        );
    }
}
