//! Global navigation defaults depend on page/root ordering and flow starts.
//! The viewer opens the first page; preview opens the first root frame across
//! pages in order. A change that can move either, or any flowStart, is global.
use super::index::PageIndex;
use crate::*;

pub(super) fn changed(
    before: &BTreeMap<&str, &Node>,
    after: &Candidate,
    ids: &BTreeSet<&str>,
    old: &PageIndex,
    new: &PageIndex,
) -> bool {
    // Existing pages keep content and relative order; new pages may only be
    // appended to a project that already had pages. Order-preserving sort-key
    // compaction is harmless.
    if !new.order.starts_with(&old.order)
        || old.order.first() != new.order.first()
        || old
            .order
            .iter()
            .any(|id| !same_page(before.get(id.as_str()).copied(), after.nodes.get(id)))
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
        if !new.starts_with(old) {
            return true;
        }
    }
    // A first root on an empty page matters only when it becomes the project's
    // first root frame, i.e. no earlier page has one.
    first_root(old) != first_root(new)
}

/// Whether an existing page is unchanged apart from its numeric sort key.
fn same_page(old: Option<&Node>, new: Option<&Node>) -> bool {
    match (old, new) {
        (Some(a), Some(b)) => {
            a.id == b.id && a.kind == b.kind && a.parent_id == b.parent_id && a.props == b.props
        }
        _ => false,
    }
}

fn first_root(index: &PageIndex) -> Option<&str> {
    index
        .order
        .iter()
        .find_map(|page| index.roots.get(page)?.first())
        .map(String::as_str)
}
