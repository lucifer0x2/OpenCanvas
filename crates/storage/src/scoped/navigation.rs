//! Global navigation defaults depend on page/root ordering and flow starts.
use crate::*;

pub(super) fn changed(before: &Candidate, after: &Candidate) -> bool {
    let pages = |candidate: &Candidate| {
        candidate
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::Page)
            .cloned()
            .collect::<Vec<_>>()
    };
    if pages(before) != pages(after) {
        return true;
    }
    for id in before
        .nodes
        .keys()
        .chain(after.nodes.keys())
        .collect::<BTreeSet<_>>()
    {
        let old = before.nodes.get(id);
        let new = after.nodes.get(id);
        if old.and_then(|n| n.props.get("flowStart")) != new.and_then(|n| n.props.get("flowStart"))
        {
            return true;
        }
    }
    for page in before.nodes.values().filter(|n| n.kind == NodeKind::Page) {
        let old = roots(before, &page.id);
        let new = roots(after, &page.id);
        // Relative order matters; Candidate::place may compact harmless gaps.
        // Existing roots must remain a prefix, and an empty page's first root
        // can change the project-wide default selection.
        if !new.starts_with(&old) || (old.is_empty() && !new.is_empty()) {
            return true;
        }
    }
    false
}

fn roots(candidate: &Candidate, page: &str) -> Vec<String> {
    let mut nodes: Vec<_> = candidate
        .nodes
        .values()
        .filter(|n| n.kind == NodeKind::Frame && n.parent_id.as_deref() == Some(page))
        .map(|n| (n.sort_key, n.id.clone()))
        .collect();
    nodes.sort();
    nodes.into_iter().map(|(_, id)| id).collect()
}
