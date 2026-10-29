//! Reference contexts keep definitions once, and authored instances as leaves.
use super::{index::TreeIndex, viewports::Computed};
use opencanvas_storage::{Node, NodeKind, Snapshot};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Definitions remain self-contained even when every instance overrides a resource.
pub(super) fn definition_nodes<'a>(s: &'a Snapshot, scope: &[(&Node, usize)]) -> Vec<&'a Node> {
    let mut needed = BTreeSet::new();
    let mut nodes = BTreeMap::new();
    let mut pending: Vec<_> = scope
        .iter()
        .filter_map(|(n, _)| n.props["componentId"].as_str())
        .collect();
    if pending.is_empty() {
        return Vec::new();
    }
    let index = TreeIndex::new(&s.nodes);
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
    let definitions = definition_nodes(s, scope);
    if definitions.is_empty() {
        return json!({});
    }
    let index = TreeIndex::new(&s.nodes);
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
                    if states {
                        if let Some(states) = super::states::differing(&resolved.props, &n.kind) {
                            value["resolvedStates"] = states;
                        }
                    }
                    value["resolvedProps"] =
                        super::compact::resolved_props(compact, &n.props, resolved.props.clone());
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
