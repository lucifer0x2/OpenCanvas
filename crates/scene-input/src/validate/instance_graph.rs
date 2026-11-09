//! Component references form a directed, bounded DAG independent of authored parents.
use super::Candidate;
use crate::{NodeKind, Result, StorageError};
use std::collections::{BTreeMap, BTreeSet};

impl Candidate {
    pub(super) fn instance_graph(&self) -> Result<()> {
        let mut graph: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for node in self.nodes.values().filter(|n| n.kind == NodeKind::Instance) {
            let Some(target) = node.props["componentId"].as_str() else {
                continue;
            };
            let mut parent = node.parent_id.as_ref().and_then(|id| self.nodes.get(id));
            for _ in 0..128 {
                let Some(node) = parent else {
                    break;
                };
                if node.props.get("component").is_some() {
                    graph.entry(&node.id).or_default().insert(target);
                }
                parent = node.parent_id.as_ref().and_then(|id| self.nodes.get(id));
            }
        }
        let mut heights = BTreeMap::new();
        for root in graph.keys() {
            visit(root, &graph, &mut BTreeSet::new(), &mut heights)?;
        }
        Ok(())
    }
}
fn visit<'a>(
    id: &'a str,
    graph: &BTreeMap<&'a str, BTreeSet<&'a str>>,
    path: &mut BTreeSet<&'a str>,
    heights: &mut BTreeMap<&'a str, usize>,
) -> Result<usize> {
    if let Some(height) = heights.get(id) {
        return Ok(*height);
    }
    if !path.insert(id) {
        return Err(StorageError::new(
            "INSTANCE_CYCLE",
            "Component reference cycle detected",
        ));
    }
    if path.len() > 8 {
        return Err(StorageError::new(
            "INSTANCE_DEPTH",
            "Component nesting exceeds 8",
        ));
    }
    let mut height = 1;
    for target in graph.get(id).into_iter().flatten() {
        height = height.max(1 + visit(target, graph, path, heights)?);
    }
    if height > 8 {
        return Err(StorageError::new(
            "INSTANCE_DEPTH",
            "Component nesting exceeds 8",
        ));
    }
    path.remove(id);
    heights.insert(id, height);
    Ok(height)
}
