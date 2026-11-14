//! Scoped notes: footprints name anchor pages; writes need guarded anchors.
use super::{
    index::{BatchScope, PageIndex},
    unsupported,
};
use crate::*;

pub(super) fn check(
    before: &Snapshot,
    candidate: &Candidate,
    operations: &[Operation],
    pages: &[String],
    scope: &BatchScope,
) -> Result<()> {
    let old: BTreeMap<_, _> = before.notes.iter().map(|n| (n.id.as_str(), n)).collect();
    for op in operations {
        let (id, anchor) = match op {
            Operation::CreateNote { id, node_id, .. } => (id, Some(node_id)),
            Operation::UpdateNote { id, node_id, .. } => (id, node_id.as_ref()),
            Operation::ReplyNote { id, .. } | Operation::DeleteNote { id } => (id, None),
            _ => continue,
        };
        // A requested anchor may be deleted later in the batch; its old page counts.
        let anchors = [
            old.get(id.as_str()).map(|n| scope.before.page(&n.node_id)),
            candidate
                .notes
                .get(id)
                .map(|n| scope.after.page(&n.node_id)),
            anchor.map(|node| scope.after.page(node).or_else(|| scope.before.page(node))),
        ];
        for page in anchors.into_iter().flatten() {
            if !page.is_some_and(|page| pages.iter().any(|p| p == page)) {
                return Err(unsupported(
                    "Note anchors (current and new) must lie on guarded pages",
                ));
            }
        }
    }
    Ok(())
}

/// A note change also names its old and new anchor pages, so guards on those
/// pages see note edits, moves, cascades and delete/recreate.
pub(super) fn footprint(
    before: &Snapshot,
    after: &Candidate,
    changed: Option<&BTreeSet<&str>>,
    old_pages: &PageIndex,
    new_pages: &PageIndex,
    keys: &mut BTreeSet<String>,
) -> Result<()> {
    let old: BTreeMap<_, _> = before.notes.iter().map(|n| (n.id.as_str(), n)).collect();
    let ids: BTreeSet<_> = match changed {
        Some(ids) => ids.clone(),
        None => old
            .keys()
            .copied()
            .chain(after.notes.keys().map(String::as_str))
            .collect(),
    };
    for id in ids {
        let (old, new) = (old.get(id).copied(), after.notes.get(id));
        if super::history::same(old, new)? {
            continue;
        }
        keys.insert(format!("note:{id}"));
        for (note, pages) in [(old, old_pages), (new, new_pages)] {
            if let Some(page) = note.and_then(|n| pages.page(&n.node_id)) {
                keys.insert(format!("page:{page}"));
            }
        }
    }
    Ok(())
}
