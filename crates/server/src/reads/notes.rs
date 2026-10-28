//! Notes in a `get_design_context` read: those anchored to the frame's ancestors (outermost
//! first), then to the frame and its descendants in preorder; one node's notes by ID.
use super::index::TreeIndex;
use opencanvas_storage::{Node, Note, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub(super) enum Filter {
    /// Unresolved notes only (the default).
    Open,
    /// Resolved notes too; they carry `resolved:true`.
    All,
    None,
}

/// `("notes", note)` page entries for the frame `scope` whose subtree is `ordered`.
pub(super) fn entries(
    s: &Snapshot,
    index: &TreeIndex,
    scope: &str,
    ordered: &[(&Node, usize)],
    filter: Option<Filter>,
) -> Vec<(String, Value)> {
    let filter = filter.unwrap_or(Filter::Open);
    if filter == Filter::None || s.notes.is_empty() {
        return Vec::new();
    }
    // Snapshot notes are ordered by ID, so each node's list stays in ID order.
    let mut by_node: HashMap<&str, Vec<&Note>> = HashMap::new();
    for note in s
        .notes
        .iter()
        .filter(|n| filter == Filter::All || !n.resolved)
    {
        by_node.entry(note.node_id.as_str()).or_default().push(note);
    }
    let ancestors = index.ancestors(scope, None);
    let nodes = ancestors
        .iter()
        .copied()
        .chain(ordered.iter().map(|(n, _)| *n));
    nodes
        .filter_map(|node| by_node.remove(node.id.as_str()))
        .flatten()
        .map(|note| ("notes".to_string(), json!(note)))
        .collect()
}
