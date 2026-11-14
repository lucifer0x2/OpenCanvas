//! Revision-local payload positions and adjacency; never owns node payloads.
use crate::{node_links, Node, NodeKind, Snapshot};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct ScopeIndex {
    pub(super) positions: BTreeMap<String, usize>,
    pub(super) children: BTreeMap<String, Vec<usize>>,
    pub(super) incoming: BTreeMap<String, Vec<usize>>,
    pub(super) roots: Vec<usize>,
    pub(super) frames: Vec<usize>,
    pub(super) notes: BTreeMap<String, Vec<usize>>,
    bound: Binding,
}

/// The snapshot an index was built for. Positions are meaningless elsewhere.
#[derive(Default, PartialEq, Eq)]
struct Binding {
    project_id: String,
    revision: u64,
    nodes: usize,
    notes: usize,
}

impl ScopeIndex {
    /// Build metadata for exactly this immutable snapshot's node vector.
    pub fn new(snapshot: &Snapshot) -> Self {
        let mut index = Self::from_nodes(&snapshot.nodes);
        index.bound = Binding::of(snapshot);
        for (position, note) in snapshot.notes.iter().enumerate() {
            index
                .notes
                .entry(note.node_id.clone())
                .or_default()
                .push(position);
        }
        index
    }

    fn from_nodes<'a>(nodes: impl IntoIterator<Item = &'a Node>) -> Self {
        let mut index = Self::default();
        for (position, node) in nodes.into_iter().enumerate() {
            index.positions.insert(node.id.clone(), position);
            if node.kind == NodeKind::Frame {
                index.frames.push(position);
            }
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

    /// Cheap guard that this index describes `snapshot`: identity, vector lengths
    /// and the first/last node positions. Mismatches must never be dereferenced.
    pub fn indexes(&self, snapshot: &Snapshot) -> bool {
        let edge = |node: Option<&Node>, at: usize| {
            node.is_none_or(|n| self.positions.get(&n.id) == Some(&at))
        };
        self.bound == Binding::of(snapshot)
            && edge(snapshot.nodes.first(), 0)
            && edge(
                snapshot.nodes.last(),
                snapshot.nodes.len().saturating_sub(1),
            )
    }

    pub fn estimated_bytes(&self) -> usize {
        (self.roots.len() + self.frames.len()) * 8
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

impl Binding {
    fn of(snapshot: &Snapshot) -> Self {
        Self {
            project_id: snapshot.project_id.clone(),
            revision: snapshot.revision,
            nodes: snapshot.nodes.len(),
            notes: snapshot.notes.len(),
        }
    }
}
