//! Borrowed authored-tree and component usage indexes. No resource/scene cloning.
use crate::{Node, NodeKind};
use std::collections::{BTreeMap, BTreeSet};

/// Index over exactly one immutable authored node set. Rebuild after mutation.
pub struct NodeIndex<'a> {
    nodes: BTreeMap<&'a str, &'a Node>,
    children: BTreeMap<&'a str, Vec<&'a str>>,
    users: BTreeMap<&'a str, Vec<&'a str>>,
}

impl<'a> NodeIndex<'a> {
    pub fn new(nodes: impl IntoIterator<Item = &'a Node>) -> Self {
        let mut index = Self {
            nodes: BTreeMap::new(),
            children: BTreeMap::new(),
            users: BTreeMap::new(),
        };
        for node in nodes {
            index.nodes.insert(&node.id, node);
            if let Some(parent) = node.parent_id.as_deref() {
                index.children.entry(parent).or_default().push(&node.id);
            }
            if node.kind == NodeKind::Instance {
                if let Some(master) = node.props["componentId"].as_str() {
                    index.users.entry(master).or_default().push(&node.id);
                }
            }
        }
        index
    }

    pub fn node(&self, id: &str) -> Option<&'a Node> {
        self.nodes.get(id).copied()
    }

    pub fn children(&self, id: &str) -> impl Iterator<Item = &'a str> + '_ {
        self.children.get(id).into_iter().flatten().copied()
    }

    pub fn component_users(&self, id: &str) -> impl Iterator<Item = &'a str> + '_ {
        self.users.get(id).into_iter().flatten().copied()
    }

    /// Component ancestors, including `id` itself when it is a master. Bounded
    /// even for an invalid candidate; validation reports its cycle/depth error.
    pub fn component_ancestors(&self, id: &str) -> Vec<&'a str> {
        let mut current = self.node(id);
        let mut masters = Vec::new();
        for _ in 0..128 {
            let Some(node) = current else { break };
            if node.props.get("component").is_some() {
                masters.push(node.id.as_str());
            }
            current = node.parent_id.as_deref().and_then(|id| self.node(id));
        }
        masters
    }

    /// Union closure: descendants and users of every enclosing master, including
    /// nested users. Both indexes are required when reparent/delete removes an edge.
    pub(super) fn affected(&self, other: &Self, seeds: BTreeSet<String>) -> BTreeSet<String> {
        let mut pending: Vec<_> = seeds.into_iter().collect();
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            for index in [self, other] {
                pending.extend(index.children(&id).map(str::to_owned));
                for master in index.component_ancestors(&id) {
                    // An instance can move across masters within a batch.
                    pending.extend(self.component_users(master).map(str::to_owned));
                    pending.extend(other.component_users(master).map(str::to_owned));
                }
            }
        }
        seen
    }
}
