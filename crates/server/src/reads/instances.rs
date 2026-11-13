//! Reference contexts keep definitions once, and authored instances as leaves.
use super::{index::TreeIndex, viewports::Computed};
use opencanvas_storage::{Node, NodeKind, Snapshot};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Definitions remain self-contained even when every instance overrides a resource.
pub(super) fn definition_nodes<'a>(s: &'a Snapshot, scope: &[(&Node, usize)]) -> Vec<&'a Node> {
    definition_nodes_indexed(&TreeIndex::new(&s.nodes), scope)
}

pub(super) fn definition_nodes_indexed<'a>(
    index: &TreeIndex<'a>,
    scope: &[(&Node, usize)],
) -> Vec<&'a Node> {
    let mut needed = BTreeSet::new();
    let mut nodes = BTreeMap::new();
    let mut pending: Vec<_> = scope
        .iter()
        .filter_map(|(n, _)| n.props["componentId"].as_str())
        .collect();
    if pending.is_empty() {
        return Vec::new();
    }
    while let Some(id) = pending.pop() {
        if !needed.insert(id) {
            continue;
        }
        let tree = index.preorder(Some(id));
        for (node, _) in &tree {
            if let Some(id) = node.props["componentId"].as_str() {
                pending.push(id);
            }
            nodes.insert(node.id.as_str(), *node);
        }
    }
    nodes.into_values().collect()
}

pub(super) fn definitions(
    s: &Snapshot,
    scope: &[(&Node, usize)],
    resolved: &HashMap<&str, &Node>,
    compact: bool,
    states: bool,
) -> Value {
    definitions_indexed(&TreeIndex::new(&s.nodes), scope, resolved, compact, states)
}

pub(super) fn definitions_indexed(
    index: &TreeIndex,
    scope: &[(&Node, usize)],
    resolved: &HashMap<&str, &Node>,
    compact: bool,
    states: bool,
) -> Value {
    let definitions = definition_nodes_indexed(index, scope);
    if definitions.is_empty() {
        return json!({});
    }
    let mut components = BTreeMap::new();
    for root in definitions
        .iter()
        .filter(|n| n.props.get("component").is_some())
    {
        let id = root.id.as_str();
        let tree = index.preorder(Some(id));
        let nodes: Vec<_> = tree
            .iter()
            .map(|(n, _)| {
                let mut value = json!(n);
                if let Some(resolved) = resolved.get(n.id.as_str()) {
                    value["effectiveTheme"] = resolved.props["effectiveTheme"].clone();
                    if states {
                        if let Some(states) = super::states::differing(&resolved.props, &n.kind) {
                            value["resolvedStates"] = states;
                        }
                    }
                    let mut props = resolved.props.clone();
                    props.as_object_mut().unwrap().remove("effectiveTheme");
                    value["resolvedProps"] =
                        super::compact::resolved_props(compact, &n.props, props);
                }
                value
            })
            .collect();
        components.insert(
            id,
            json!({"name":root.props["component"]["name"],"rootId":id,"nodes":nodes}),
        );
    }
    json!(components)
}

pub(super) fn reference(
    node: &Node,
    value: &mut Value,
    expanded: &TreeIndex,
    computed: Option<&Computed>,
) {
    if node.kind != NodeKind::Instance {
        return;
    }
    value["componentId"] = node.props["componentId"].clone();
    value["childCount"] = json!(expanded.preorder(Some(&node.id)).len().saturating_sub(1));
    if let Some(overrides) = node.props.get("overrides") {
        value["overrides"] = overrides.clone();
    }
    // Resolved master properties belong in its definition, not repeated per reference.
    value.as_object_mut().unwrap().remove("resolvedProps");
    if let Some(computed) = computed {
        let children: BTreeMap<_, _> = expanded.preorder(Some(&node.id)).into_iter().skip(1)
            .map(|(child, _)| (child.id.clone(), json!({"bounds":computed.bounds.get(&child.id),"lines":computed.lines.get(&child.id)}))).collect();
        value["computed"] = json!({"bounds":computed.bounds.get(&node.id),"children":children});
    }
}
