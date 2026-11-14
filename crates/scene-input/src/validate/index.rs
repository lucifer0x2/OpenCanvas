//! Borrowed authored-tree and component usage indexes. No resource/scene cloning.
use super::NodeTopology;
use crate::Node;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Index over exactly one immutable authored node set. Rebuild after mutation.
pub struct NodeIndex<'a> {
    nodes: BTreeMap<&'a str, &'a Node>,
    topology: Arc<NodeTopology>,
}

impl<'a> NodeIndex<'a> {
    pub fn new(nodes: impl IntoIterator<Item = &'a Node>) -> Self {
        let nodes: Vec<_> = nodes.into_iter().collect();
        let topology = Arc::new(NodeTopology::new(nodes.iter().copied()));
        Self::with_topology(nodes, topology)
    }

    pub fn with_topology(
        nodes: impl IntoIterator<Item = &'a Node>,
        topology: Arc<NodeTopology>,
    ) -> Self {
        Self {
            nodes: nodes
                .into_iter()
                .map(|node| (node.id.as_str(), node))
                .collect(),
            topology,
        }
    }

    pub fn node(&self, id: &str) -> Option<&'a Node> {
        self.nodes.get(id).copied()
    }

    pub fn children(&self, id: &str) -> impl Iterator<Item = &'a str> + '_ {
        self.topology
            .children(id)
            .filter_map(|id| self.nodes.get(id).map(|node| node.id.as_str()))
    }

    pub fn component_users(&self, id: &str) -> impl Iterator<Item = &'a str> + '_ {
        self.topology
            .component_users(id)
            .filter_map(|id| self.nodes.get(id).map(|node| node.id.as_str()))
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
