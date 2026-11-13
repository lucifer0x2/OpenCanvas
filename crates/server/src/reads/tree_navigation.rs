//! A separate shallow index keeps incoming edges outside the requested tree.
use super::{index::TreeIndex, navigation};
use opencanvas_storage::{
    read_scope::{ScopedNodes, ScopedSnapshot},
    Node,
};
use serde_json::Value;
use std::collections::HashMap;

pub(super) fn summary(
    scoped: &ScopedSnapshot<'_>,
    nodes: &ScopedNodes<'_>,
    frame: &str,
) -> Option<Value> {
    let mut authored: HashMap<&str, &Node> =
        nodes.authored.iter().map(|n| (n.id.as_str(), *n)).collect();
    authored.extend(
        scoped
            .navigation_nodes(&nodes.authored, frame)
            .into_iter()
            .map(|n| (n.id.as_str(), n)),
    );
    let mut authored: Vec<_> = authored.into_values().collect();
    // All references belong to one immutable source slice: addresses retain its
    // source ordering without rescanning unrelated nodes.
    authored.sort_by_key(|n| *n as *const Node as usize);
    let nav = TreeIndex::from_nodes(authored);
    navigation::summary(&nav, frame)
}
