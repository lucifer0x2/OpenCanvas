//! Bounded reads over an immutable, explicitly validated authored revision.
mod closure;
mod index;
mod input;
mod navigation;

use crate::{validate::Candidate, Node, Note, Result, Snapshot};
pub use index::ScopeIndex;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub enum ReadScope<'a> {
    Page(&'a str),
    Target(&'a str),
}

/// A borrowed read capability. Standalone/untrusted callers must use `validate`.
pub struct ScopedSnapshot<'a> {
    snapshot: &'a Snapshot,
    index: Arc<ScopeIndex>,
}

/// Source-order authored/expanded vectors, with ancestors needed for placed themes.
/// Definitions are included only when requested. Resources remain on the snapshot.
pub struct ScopedNodes<'a> {
    pub authored: Vec<&'a Node>,
    pub dependencies: Vec<&'a Node>,
    pub expanded: Vec<Node>,
    pub stats: ScopeStats,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScopeStats {
    pub closure_nodes: usize,
    pub expanded_nodes: usize,
    pub resolved_nodes: usize,
}

impl<'a> ScopedSnapshot<'a> {
    pub fn validate(snapshot: &'a Snapshot) -> Result<Self> {
        Candidate::from_snapshot(snapshot).validate()?;
        Ok(Self::from_validated(
            snapshot,
            Arc::new(ScopeIndex::new(snapshot)),
        ))
    }

    /// Explicit trusted boundary for storage adapters with a validated-write invariant.
    /// The caller guarantees the *entire* snapshot already passed full validation
    /// and that `index` describes this exact immutable node vector in source order.
    /// Cache identity alone does not establish either guarantee. This deliberately
    /// does not change standalone resolution/validation APIs.
    pub fn from_validated(snapshot: &'a Snapshot, index: Arc<ScopeIndex>) -> Self {
        Self { snapshot, index }
    }

    pub fn snapshot(&self) -> &'a Snapshot {
        self.snapshot
    }

    pub fn node(&self, id: &str) -> Option<&'a Node> {
        self.index
            .positions
            .get(id)
            .map(|&i| &self.snapshot.nodes[i])
    }

    /// Top-level authored roots in source order. Apply stable sibling sortKey
    /// ordering at the read layer, exactly as for the full source vector.
    pub fn roots(&self) -> impl Iterator<Item = &'a Node> + '_ {
        self.index.roots.iter().map(|&i| &self.snapshot.nodes[i])
    }

    /// Notes anchored to this authored dependency closure, in snapshot note order.
    /// Include `ScopedNodes::dependencies` for intent on virtual source/master IDs.
    pub fn notes_for_nodes<'id>(&self, ids: impl IntoIterator<Item = &'id str>) -> Vec<&'a Note> {
        let positions: std::collections::BTreeSet<_> = ids
            .into_iter()
            .flat_map(|id| self.index.notes.get(id).into_iter().flatten().copied())
            .collect();
        positions
            .into_iter()
            .map(|i| &self.snapshot.notes[i])
            .collect()
    }

    /// Resolve only these expanded nodes, reusing full immutable resource metadata.
    /// The vector retains expansion order, IDs, visibility and placed-theme semantics.
    pub fn resolve(&self, nodes: &mut ScopedNodes<'_>, theme: &str) -> Result<Vec<Node>> {
        let result = crate::validate::resolve_scoped(self.snapshot, &nodes.expanded, theme)?;
        nodes.stats.resolved_nodes = result.len();
        Ok(result)
    }

    /// Full token resource responses retain their existing contract without node work.
    pub fn resolved_tokens(&self, theme: &str) -> Result<BTreeMap<String, serde_json::Value>> {
        crate::validate::resolve_scoped_tokens(self.snapshot, theme)
    }
}
