//! Revision-local payload positions and adjacency; never owns node payloads.
use crate::{node_links, Node, Snapshot};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct ScopeIndex {
    pub(super) positions: BTreeMap<String, usize>,
    pub(super) children: BTreeMap<String, Vec<usize>>,
    pub(super) incoming: BTreeMap<String, Vec<usize>>,
    pub(super) roots: Vec<usize>,
    pub(super) notes: BTreeMap<String, Vec<usize>>,
}

impl ScopeIndex {
    /// Build metadata for exactly this immutable snapshot's node vector.
    pub fn new(snapshot: &Snapshot) -> Self {
        let mut index = Self::from_nodes(&snapshot.nodes);
        for (position, note) in snapshot.notes.iter().enumerate() {
            index
                .notes
                .entry(note.node_id.clone())
                .or_default()
                .push(position);
        }
        index
    }

    pub fn from_nodes<'a>(nodes: impl IntoIterator<Item = &'a Node>) -> Self {
        let mut index = Self::default();
        for (position, node) in nodes.into_iter().enumerate() {
            index.positions.insert(node.id.clone(), position);
            if let Some(parent) = &node.parent_id {
                index
                    .children
                    .entry(parent.clone())
                    .or_default()
                    .push(position);
            } else {
                index.roots.push(position);
            }
            for link in node_links(&node.props) {
                if let Some(target) = link.to {
                    index.incoming.entry(target).or_default().push(position);
                }
            }
        }
        index
    }

    pub fn estimated_bytes(&self) -> usize {
        self.roots.len() * 8
            + self.positions.keys().map(|id| 64 + id.len()).sum::<usize>()
            + self
                .children
                .iter()
                .chain(&self.incoming)
                .chain(&self.notes)
                .map(|(id, list)| 96 + id.len() + list.len() * 8)
                .sum::<usize>()
    }
}
