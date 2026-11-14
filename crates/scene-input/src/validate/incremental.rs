//! Commit validation from a fully valid before snapshot; pure reads/imports use
//! `Candidate::validate` and never rely on this write-side precondition.
use super::{scope::Resolver, Candidate, NodeIndex, NodeTopology};
use crate::{
    bench_phase::time, Node, NodeKind, Operation, Result, Snapshot, StorageError, MAX_NODES,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Derived work for this candidate only. Reuse expansion downstream instead of
/// expanding the same candidate again. No persisted/public response contract.
pub struct ValidationWork {
    pub affected_ids: BTreeSet<String>,
    pub expanded: Option<BTreeMap<String, Node>>,
    pub stats: ValidationStats,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ValidationStats {
    pub authored_nodes: usize,
    pub affected_nodes: usize,
    pub expanded_nodes: usize,
    pub affected_virtual_nodes: usize,
    pub full_fallback: bool,
}

impl Candidate {
    /// Validate a batch's final candidate. `before` MUST have passed full
    /// validation; candidate must come from applying `operations` to it, with
    /// `changed` containing every mutation (including cascades/link pruning).
    /// Never use this method to validate a raw bundle or untrusted/corrupt read.
    /// Resource/page/note writes conservatively use the full reference path.
    pub fn validate_after(
        &self,
        before: &Snapshot,
        operations: &[Operation],
        changed: &BTreeSet<String>,
    ) -> Result<ValidationWork> {
        self.validate_after_indexed(
            before,
            operations,
            changed,
            Arc::new(NodeTopology::new(&before.nodes)),
            Arc::new(NodeTopology::new(self.nodes.values())),
        )
    }

    /// Both topologies must describe the supplied immutable node sets exactly.
    pub fn validate_after_indexed(
        &self,
        before: &Snapshot,
        operations: &[Operation],
        changed: &BTreeSet<String>,
        old: Arc<NodeTopology>,
        new: Arc<NodeTopology>,
    ) -> Result<ValidationWork> {
        let old = time("validate.index.before", || {
            NodeIndex::with_topology(&before.nodes, old)
        });
        let new = time("validate.index.after", || {
            NodeIndex::with_topology(self.nodes.values(), new)
        });
        if needs_full(operations, &old, &new) || resources_changed(self, before) {
            self.validate()?;
            return Ok(ValidationWork {
                affected_ids: self.nodes.keys().cloned().collect(),
                expanded: None,
                stats: ValidationStats {
                    authored_nodes: self.nodes.len(),
                    affected_nodes: self.nodes.len(),
                    full_fallback: true,
                    ..Default::default()
                },
            });
        }
        let affected = time("validate.affected", || {
            let mut seeds = changed.clone();
            for op in operations {
                let id = match op {
                    Operation::CreateNode { id, .. }
                    | Operation::ReparentNode { id, .. }
                    | Operation::ReorderNode { id, .. }
                    | Operation::DeleteSubtree { id } => id,
                    _ => continue,
                };
                for index in [&old, &new] {
                    if let Some(parent) = index.node(id).and_then(|n| n.parent_id.as_ref()) {
                        seeds.insert(parent.clone());
                    }
                }
            }
            old.affected(&new, seeds)
        });
        if self.nodes.len() > MAX_NODES {
            return Err(StorageError::validation(
                "nodes",
                "Project exceeds 20000 authored nodes",
            ));
        }
        time("validate.notes_navigation", || {
            self.validate_notes()?;
            self.validate_navigation()
        })?;
        let themes = time("validate.tokens", || self.validate_tokens())?;
        for theme in &themes {
            time("validate.theme", || -> Result<()> {
                let context = Resolver {
                    candidate: self,
                    nodes: &self.nodes,
                    theme,
                };
                // Unchanged styles already passed every theme; resource writes fall back.
                for id in &affected {
                    if let Some(node) = self.nodes.get(id) {
                        context.node(node)?;
                    }
                }
                Ok(())
            })?;
        }
        time("validate.animation_triggers", || {
            self.validate_animation_triggers()
        })?;
        let expanded = time("validate.instances", || {
            self.validate_instances(Some(&affected), &themes)
        })?;
        let stats = ValidationStats {
            authored_nodes: self.nodes.len(),
            affected_nodes: affected
                .iter()
                .filter(|id| self.nodes.contains_key(*id))
                .count(),
            expanded_nodes: expanded.as_ref().map_or(self.nodes.len(), BTreeMap::len),
            affected_virtual_nodes: expanded.as_ref().map_or(0, |nodes| {
                nodes
                    .keys()
                    .filter(|id| {
                        id.contains('/') && affected.contains(id.split('/').next().unwrap())
                    })
                    .count()
            }),
            full_fallback: false,
        };
        Ok(ValidationWork {
            affected_ids: affected,
            expanded,
            stats,
        })
    }
}

fn needs_full(operations: &[Operation], old: &NodeIndex<'_>, new: &NodeIndex<'_>) -> bool {
    operations.iter().any(|op| {
        let id = match op {
            Operation::CreateNode {
                id, kind, props, ..
            } => {
                if *kind == NodeKind::Page || props.get("theme").is_some() {
                    return true;
                }
                id
            }
            Operation::UpdateNode { id, props } => {
                if props.get("theme").is_some() {
                    return true;
                }
                id
            }
            Operation::ReparentNode { id, .. }
            | Operation::ReorderNode { id, .. }
            | Operation::DeleteSubtree { id } => id,
            // Explicit exhaustive classifier: new resource/operation variants must
            // make a deliberate choice here rather than silently opt in.
            Operation::UpsertToken { .. }
            | Operation::UpsertStyle { .. }
            | Operation::CreateNote { .. }
            | Operation::ReplyNote { .. }
            | Operation::UpdateNote { .. }
            | Operation::DeleteNote { .. } => return true,
        };
        [old, new]
            .iter()
            .any(|index| index.node(id).is_some_and(|n| n.kind == NodeKind::Page))
    })
}

fn resources_changed(candidate: &Candidate, before: &Snapshot) -> bool {
    macro_rules! differs {
        ($field:ident) => {
            candidate.$field.len() != before.$field.len()
                || before
                    .$field
                    .iter()
                    .any(|value| candidate.$field.get(&value.id) != Some(value))
        };
    }
    differs!(tokens) || differs!(styles) || differs!(fonts) || differs!(assets) || differs!(notes)
}
