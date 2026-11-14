//! Immutable authored adjacency, reusable without borrowing or cloning node payloads.
use crate::{Node, NodeKind};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct NodeTopology {
    pub(crate) children: BTreeMap<String, Vec<String>>,
    pub(crate) users: BTreeMap<String, Vec<String>>,
}

impl NodeTopology {
    pub fn new<'a>(nodes: impl IntoIterator<Item = &'a Node>) -> Self {
        let mut result = Self::default();
        for node in nodes {
            if let Some(parent) = &node.parent_id {
                result
                    .children
                    .entry(parent.clone())
                    .or_default()
                    .push(node.id.clone());
            }
            if node.kind == NodeKind::Instance {
                if let Some(master) = node.props["componentId"].as_str() {
                    result
                        .users
                        .entry(master.into())
                        .or_default()
                        .push(node.id.clone());
                }
            }
        }
        result
    }

    pub fn children(&self, id: &str) -> impl Iterator<Item = &str> {
        self.children
            .get(id)
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    pub fn component_users(&self, id: &str) -> impl Iterator<Item = &str> {
        self.users.get(id).into_iter().flatten().map(String::as_str)
    }

    /// Property-only edits preserve topology unless component identity changes.
    pub fn preserves(operations: &[crate::Operation], changed: &BTreeSet<String>) -> bool {
        operations.iter().all(|op| match op {
            crate::Operation::UpdateNode { props, .. } => {
                props.get("componentId").is_none() && props.get("component").is_none()
            }
            _ => false,
        }) && changed.len() <= operations.len()
    }

    pub fn estimated_bytes(&self) -> usize {
        self.children
            .iter()
            .chain(&self.users)
            .map(|(key, ids)| 96 + key.len() + ids.iter().map(|id| 24 + id.len()).sum::<usize>())
            .sum()
    }
}
