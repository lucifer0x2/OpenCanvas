#[allow(dead_code)]
mod incremental_support;
mod scoped_reference;
use incremental_support::*;
use opencanvas_scene_input::{read_scope::*, *};
use serde_json::json;

fn parity(snapshot: &Snapshot, request: ReadScope<'_>, definitions: bool) -> ScopeStats {
    let scoped = ScopedSnapshot::validate(snapshot).unwrap();
    let mut nodes = scoped.expand(request, definitions).unwrap();
    // Completeness: no dropped authored or virtual node, independent of the scoped path.
    assert_eq!(
        nodes.expanded.iter().map(|n| &n.id).collect::<Vec<_>>(),
        scoped_reference::reference_ids(snapshot, request, definitions)
            .iter()
            .collect::<Vec<_>>(),
        "{request:?} definitions={definitions}"
    );
    for theme in ["default", "night"] {
        let resolved = scoped.resolve(&mut nodes, theme).unwrap();
        let all = resolved_expanded_nodes(snapshot, theme).unwrap();
        let ids: std::collections::BTreeSet<_> = nodes.expanded.iter().map(|n| &n.id).collect();
        let expected: Vec<_> = all.into_iter().filter(|n| ids.contains(&n.id)).collect();
        assert_eq!(
            serde_json::to_vec(&resolved).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
        assert_eq!(
            scoped.resolved_tokens(theme).unwrap(),
            resolved_tokens_for_theme(snapshot, theme).unwrap()
        );
    }
    let full = instances::expand(&snapshot.nodes).unwrap();
    let expected: Vec<_> = full
        .into_iter()
        .filter(|n| nodes.expanded.iter().any(|s| s.id == n.id))
        .collect();
    assert_eq!(nodes.expanded, expected);
    nodes.stats
}

#[test]
fn page_target_and_virtual_closures_match_full_placed_and_definition_resolution() {
    let mut snapshot = fixture(3, 1);
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "instance0")
        .unwrap()
        .props["overrides"] = json!({"nested":{"fill":"#123456"}});
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "label")
        .unwrap()
        .props["fontFamily"] = json!("Fixture");
    for request in [
        ReadScope::Page("page0"),
        ReadScope::Target("frame0"),
        ReadScope::Target("instance0"),
        ReadScope::Target("instance0/nested/label"),
    ] {
        parity(&snapshot, request, false);
        parity(&snapshot, request, true);
    }
}

#[test]
fn scoped_expansion_and_resolution_work_is_independent_of_unrelated_pages() {
    let small = fixture(1, 0);
    let large = fixture(25, 50);
    let scope = ReadScope::Target("instance0");
    let a = parity(&small, scope, true);
    let b = parity(&large, scope, true);
    assert_eq!(a, b);
    assert!(b.expanded_nodes < 30);
    eprintln!("scoped counts small={a:?}, unrelated-growth={b:?}; construction excluded");
    assert_eq!(
        parity(&small, ReadScope::Target("child0"), false),
        parity(&large, ReadScope::Target("child0"), false)
    );
}

#[test]
fn source_order_and_equal_sort_keys_survive_non_topological_node_vectors() {
    let mut snapshot = fixture(2, 3);
    snapshot.nodes.reverse();
    for node in &mut snapshot.nodes {
        node.sort_key = 9;
    }
    parity(&snapshot, ReadScope::Page("page0"), true);
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    assert_eq!(
        scoped.roots().map(|n| &n.id).collect::<Vec<_>>(),
        snapshot
            .nodes
            .iter()
            .filter(|n| n.parent_id.is_none())
            .map(|n| &n.id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn navigation_metadata_includes_cross_page_names_incoming_and_screen_ancestry_only() {
    let mut snapshot = fixture(3, 3);
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "frame1")
        .unwrap()
        .props["name"] = json!("Destination");
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "child0")
        .unwrap()
        .props["links"] = json!([{"to":"frame1"}]);
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "child2")
        .unwrap()
        .props["links"] = json!([{"to":"frame0"}]);
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    let nodes = scoped.expand(ReadScope::Target("frame0"), false).unwrap();
    let navigation = scoped.navigation_nodes(&nodes.authored, "frame0");
    let ids: Vec<_> = navigation.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        ["page1", "frame1", "page2", "frame2", "group2", "child2"]
    );
    assert!(!ids.contains(&"extra1_0"));
    assert_eq!(scoped.node("frame1").unwrap().props["name"], "Destination");
    parity(&snapshot, ReadScope::Page("page0"), true);
}

#[test]
fn untrusted_scope_keeps_full_validation_and_serialized_error_precedence() {
    let mut invalid = fixture(2, 0);
    invalid
        .nodes
        .iter_mut()
        .find(|n| n.id == "text1")
        .unwrap()
        .props["fontId"] = json!("missing");
    let reference = resolved_expanded_nodes(&invalid, "absent").unwrap_err();
    let scoped = ScopedSnapshot::validate(&invalid).err().unwrap();
    assert_eq!(
        serde_json::to_value(scoped).unwrap(),
        serde_json::to_value(reference).unwrap()
    );
    let valid = fixture(1, 0);
    let scoped = ScopedSnapshot::validate(&valid).unwrap();
    for id in ["missing", "instance0/missing", "instance0/nested/missing"] {
        assert!(scoped.expand(ReadScope::Target(id), false).is_err());
    }
    assert!(scoped.expand(ReadScope::Page("frame0"), false).is_err());
}

#[test]
fn maximum_valid_ancestry_keeps_page_in_target_and_navigation_metadata() {
    let mut snapshot = fixture(1, 0);
    let mut parent = "frame0".to_string();
    for i in 0..125 {
        let id = format!("deep{i}");
        snapshot
            .nodes
            .push(node(&id, NodeKind::Group, Some(&parent), json!({})));
        parent = id;
    }
    snapshot.nodes.push(node(
        "deep_link",
        NodeKind::Rectangle,
        Some(&parent),
        json!({"links":[{"to":"master"}]}),
    ));
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    let nodes = scoped
        .expand(ReadScope::Target("deep_link"), false)
        .unwrap();
    assert_eq!(nodes.authored.len(), 128);
    assert!(nodes.authored.iter().any(|n| n.id == "page0"));
    let nav = scoped.navigation_nodes(&[], "master");
    assert_eq!(nav.len(), 128);
    assert!(nav.iter().any(|n| n.id == "page0"));
}

#[test]
fn indexed_notes_keep_source_order_and_virtual_source_master_intent_dependencies() {
    let mut snapshot = fixture(10, 1);
    snapshot.notes = [
        ("z", "label"),
        ("a", "page0"),
        ("m", "master"),
        ("other", "text9"),
    ]
    .into_iter()
    .map(|(id, anchor)| Note {
        id: id.into(),
        node_id: anchor.into(),
        text: "intentional overflow: fixture".into(),
        author: None,
        resolved: false,
        replies: vec![],
    })
    .collect();
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    let nodes = scoped
        .expand(ReadScope::Target("instance0"), false)
        .unwrap();
    assert!(!nodes.authored.iter().any(|n| n.id == "label"));
    assert!(nodes.dependencies.iter().any(|n| n.id == "label"));
    let notes = scoped.notes_for_nodes(nodes.dependencies.iter().map(|n| n.id.as_str()));
    assert_eq!(
        notes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
        ["z", "a", "m"]
    );
    assert_eq!(scoped.notes_for_nodes(["label", "label"]).len(), 1);
    assert!(scoped.notes_for_nodes(["missing"]).is_empty());
}
