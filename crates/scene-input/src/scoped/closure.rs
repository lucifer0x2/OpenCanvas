use super::{ReadScope, ScopeStats, ScopedNodes, ScopedSnapshot};
use crate::{node_links, Node, NodeKind, Result, StorageError};
use std::collections::BTreeSet;

impl<'a> ScopedSnapshot<'a> {
    /// Iterative: corrupt stored rows can form arbitrarily deep chains.
    fn descendants(&self, id: &str, selected: &mut BTreeSet<usize>) {
        let mut stack: Vec<usize> = self.index.positions.get(id).copied().into_iter().collect();
        while let Some(position) = stack.pop() {
            if !selected.insert(position) {
                continue;
            }
            let id = &self.snapshot.nodes[position].id;
            stack.extend(self.index.children.get(id).into_iter().flatten());
        }
    }

    fn ancestors(&self, position: usize, selected: &mut BTreeSet<usize>) {
        let mut position = Some(position);
        for _ in 0..=self.snapshot.nodes.len() {
            let Some(current) = position else { break };
            selected.insert(current);
            position = self.snapshot.nodes[current]
                .parent_id
                .as_deref()
                .and_then(|id| self.index.positions.get(id).copied());
        }
    }

    /// Include transitive masters as lookup dependencies. `definitions` additionally
    /// expands/resolves their authored trees for MCP reference-mode component metadata.
    /// Target IDs may be virtual; only their outer authored instance is expanded,
    /// rather than every frame on the containing page.
    pub fn expand(&self, scope: ReadScope<'_>, definitions: bool) -> Result<ScopedNodes<'a>> {
        let (id, page) = match scope {
            ReadScope::Page(id) => (id, true),
            ReadScope::Target(id) => (id, false),
        };
        let authored_id = id.split('/').next().unwrap_or(id);
        let root = self
            .node(authored_id)
            .ok_or_else(|| StorageError::new("INVALID_REQUEST", "Scope node does not exist"))?;
        if page && root.kind != NodeKind::Page {
            return Err(StorageError::new(
                "INVALID_REQUEST",
                "Scope node must identify a page",
            ));
        }
        let mut selected = BTreeSet::new();
        self.descendants(authored_id, &mut selected);
        self.ancestors(self.index.positions[authored_id], &mut selected);
        let mut closure = selected.clone();
        let mut scanned = BTreeSet::new();
        loop {
            let pending: Vec<_> = closure.difference(&scanned).copied().collect();
            if pending.is_empty() {
                break;
            }
            for position in pending {
                scanned.insert(position);
                let node = &self.snapshot.nodes[position];
                if let Some(master) = node.props["componentId"].as_str() {
                    self.descendants(master, &mut closure);
                    if let Some(&position) = self.index.positions.get(master) {
                        self.ancestors(position, &mut closure);
                    }
                }
            }
        }
        if definitions {
            selected = closure.clone();
        }
        let authored: Vec<_> = selected.iter().map(|&i| &self.snapshot.nodes[i]).collect();
        let dependencies: Vec<_> = closure.iter().map(|&i| &self.snapshot.nodes[i]).collect();
        let expanded = crate::instances::expand_selected(&dependencies, &authored)?;
        if !expanded.iter().any(|n| n.id == id) {
            return Err(StorageError::new(
                "INVALID_REQUEST",
                "Scope node does not exist",
            ));
        }
        let stats = ScopeStats {
            closure_nodes: closure.len(),
            expanded_nodes: expanded.len(),
            resolved_nodes: 0,
        };
        Ok(ScopedNodes {
            authored,
            dependencies,
            expanded,
            stats,
        })
    }

    /// Shallow navigation metadata: outgoing target names and indexed incoming
    /// sources/screens. It does not expand destination frames or unrelated pages.
    pub fn navigation_nodes(&self, nodes: &[&Node], frame: &str) -> Vec<&'a Node> {
        let mut selected = BTreeSet::new();
        for node in nodes {
            for link in node_links(&node.props) {
                if let Some(position) = link
                    .to
                    .as_deref()
                    .and_then(|id| self.index.positions.get(id))
                {
                    self.ancestors(*position, &mut selected);
                }
            }
        }
        for &position in self.index.incoming.get(frame).into_iter().flatten() {
            self.ancestors(position, &mut selected);
        }
        selected
            .into_iter()
            .map(|i| &self.snapshot.nodes[i])
            .collect()
    }
}
