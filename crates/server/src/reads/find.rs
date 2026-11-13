//! Exact-total search, expanding one authored root/page closure at a time.
mod criteria;
mod entry;
mod legacy;
mod page;
#[cfg(test)]
mod tests;

use super::{index::TreeIndex, pagination, ReadRequest};
use crate::error::{ApiError, Result};
use criteria::Criteria;
use opencanvas_storage::{
    read_scope::{ReadScope, ScopedSnapshot},
    NodeKind, SharedSnapshot,
};
use serde_json::{json, Value};

const RENAME_BYTES: usize = "matches".len() - "nodes".len();

pub(super) fn read_shared(s: &SharedSnapshot, req: ReadRequest) -> Result<Value> {
    match s.trusted_scoped()? {
        Some(scoped) => read_scoped(&scoped, req).map(|(value, _)| value),
        None => legacy::read(&s.snapshot, req),
    }
}

/// Work counts exclude SQL loading and cold revision-index construction.
#[derive(Debug, Default)]
struct Work {
    /// Authored nodes selected across roots; master libraries are not re-walked.
    selected: usize,
    visited: usize,
    max_expanded: usize,
    materialized_matches: usize,
    retained_matches: usize,
}

fn read_scoped(scoped: &ScopedSnapshot<'_>, req: ReadRequest) -> Result<(Value, Work)> {
    let criteria = Criteria::new(&req)?;
    if req.node_id.is_some() && req.frame_id.is_some() {
        return Err(ApiError::invalid("Choose one scope ID"));
    }
    let s = scoped.snapshot();
    let (scope, offset) = pagination::scope(s, "find_nodes", &req)?;
    let root = req.node_id.as_deref().or(req.frame_id.as_deref());
    let limit = req.limit.unwrap_or(50);
    let budget = req.budget.unwrap_or(65536);
    let mut matches = page::Matches::new(offset, limit, budget - RENAME_BYTES);
    let mut work = Work::default();
    if let Some(id) = root {
        visit(scoped, id, root, &req, &criteria, &mut matches, &mut work)?;
    } else {
        // Source order is the tie breaker, matching TreeIndex's stable root sort.
        let mut roots: Vec<_> = scoped.roots().collect();
        roots.sort_by_key(|n| n.sort_key);
        for node in roots {
            visit(
                scoped,
                &node.id,
                None,
                &req,
                &criteria,
                &mut matches,
                &mut work,
            )?;
        }
    }
    work.materialized_matches = matches.materialized;
    work.retained_matches = matches.retained();
    for (name, value) in [
        ("find.selected_nodes", work.selected),
        ("find.visited_nodes", work.visited),
        ("find.max_expanded_nodes", work.max_expanded),
        ("find.materialized_matches", work.materialized_matches),
        ("find.retained_matches", work.retained_matches),
    ] {
        opencanvas_storage::bench_phase::count(name, value);
    }
    let response = json!({"schemaVersion":1,"projectId":s.project_id,"revision":s.revision,"scope":root,"criteria":entry::echo(&req),"scannedNodes":work.visited,"totalMatches":matches.total,"nodes":[],"limits":{"nodes":limit,"bytes":budget}});
    let mut response = matches.finish(response, &scope)?;
    let found = response.as_object_mut().unwrap().remove("nodes").unwrap();
    response["matches"] = found;
    Ok((response, work))
}

fn visit(
    scoped: &ScopedSnapshot<'_>,
    id: &str,
    root: Option<&str>,
    req: &ReadRequest,
    criteria: &Criteria<'_>,
    matches: &mut page::Matches,
    work: &mut Work,
) -> Result<()> {
    let (mut expanded, stats) = scoped.expand_nodes(ReadScope::Target(id))?;
    work.selected += stats.closure_nodes;
    work.max_expanded = work.max_expanded.max(expanded.len());
    // Authored instances retain their searchable kind/props; virtual children do not.
    for node in &mut expanded {
        if let Some(authored) = scoped
            .node(&node.id)
            .filter(|n| n.kind == NodeKind::Instance)
        {
            *node = authored.clone();
        }
    }
    let index = TreeIndex::new(&expanded);
    let target = index
        .get(id)
        .ok_or_else(|| ApiError::invalid("Scope node does not exist"))?;
    if root.is_some() && req.frame_id.is_some() && target.kind != NodeKind::Frame {
        return Err(ApiError::invalid("frameId must identify a frame"));
    }
    for (node, depth) in index.preorder(Some(id)) {
        work.visited += 1;
        if let Some(on) = criteria.matched_on(node) {
            matches.push(|| entry::entry(&index, root, node, depth, on, criteria, req))?;
        }
    }
    Ok(())
}
