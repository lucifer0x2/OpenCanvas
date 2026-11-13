//! Global navigation defaults depend on page/root ordering and flow starts.
use super::index::PageIndex;
use crate::*;

pub(super) fn changed(
    before: &BTreeMap<&str, &Node>,
    after: &Candidate,
    ids: &BTreeSet<&str>,
    old: &PageIndex,
    new: &PageIndex,
) -> bool {
    if !old
        .page_nodes
        .keys()
        .filter_map(|id| before.get(id.as_str()).copied())
        .eq(new.page_nodes.keys().filter_map(|id| after.nodes.get(id)))
    {
        return true;
    }
    let mut affected_pages = BTreeSet::new();
    for id in ids {
        let old_node = before.get(id).copied();
        let new_node = after.nodes.get(*id);
        if old_node.and_then(|n| n.props.get("flowStart"))
            != new_node.and_then(|n| n.props.get("flowStart"))
        {
            return true;
        }
        for index in [old, new] {
            if let Some(page) = index.page(id) {
                affected_pages.insert(page);
            }
        }
    }
    for page in affected_pages {
        let old = old.roots.get(page).map(Vec::as_slice).unwrap_or_default();
        let new = new.roots.get(page).map(Vec::as_slice).unwrap_or_default();
        // Relative order matters; Candidate::place may compact harmless gaps.
        // Existing roots must remain a prefix, and an empty page's first root
        // can change the project-wide default selection.
        if !new.starts_with(old) || (old.is_empty() && !new.is_empty()) {
            return true;
        }
    }
    false
}
