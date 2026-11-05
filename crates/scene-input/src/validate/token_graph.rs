use super::{
    tokens::{alias, branches, MAX_ALIAS_DEPTH},
    Candidate,
};
use crate::error::*;
use std::collections::{BTreeMap, BTreeSet};

impl Candidate {
    /// Check the union of every branch, including unused themes and tokens.
    /// This deliberately rejects cross-theme cycles too, before resolution.
    pub(super) fn validate_token_graph(&self) -> Result<()> {
        let mut graph = BTreeMap::new();
        for token in self.tokens.values() {
            let mut edges = BTreeSet::new();
            for (_, value) in branches(token)? {
                if let Some(target) = alias(value)? {
                    edges.insert(target);
                }
            }
            graph.insert(token.id.as_str(), edges);
        }
        let mut heights = BTreeMap::new();
        for id in graph.keys() {
            visit(id, &graph, &mut BTreeSet::new(), &mut heights)?;
        }
        Ok(())
    }
}

fn visit<'a>(
    id: &'a str,
    graph: &BTreeMap<&'a str, BTreeSet<&'a str>>,
    active: &mut BTreeSet<&'a str>,
    heights: &mut BTreeMap<&'a str, usize>,
) -> Result<usize> {
    if active.contains(id) {
        return Err(StorageError::validation(
            "token.aliasId",
            "Token alias cycle detected across theme branches",
        ));
    }
    if let Some(height) = heights.get(id) {
        return Ok(*height);
    }
    if active.len() > MAX_ALIAS_DEPTH {
        return Err(StorageError::validation(
            "token.aliasId",
            "Token alias depth exceeds 32",
        ));
    }
    active.insert(id);
    let mut height = 0;
    for target in graph.get(id).into_iter().flatten() {
        height = height.max(visit(target, graph, active, heights)? + 1);
    }
    active.remove(id);
    if height > MAX_ALIAS_DEPTH {
        return Err(StorageError::validation(
            "token.aliasId",
            "Token alias depth exceeds 32",
        ));
    }
    heights.insert(id, height);
    Ok(height)
}
