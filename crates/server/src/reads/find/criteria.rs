use super::super::ReadRequest;
use crate::error::{ApiError, Result};
use opencanvas_storage::{Node, NodeKind};
use std::collections::HashSet;

/// Requested criteria, ANDed. Substring needles are lowercased once (no accent folding).
pub(super) struct Criteria<'r> {
    kinds: Option<&'r [String]>,
    name: Option<String>,
    pub text: Option<String>,
    ids: Option<HashSet<&'r str>>,
    parent: Option<&'r str>,
}

impl<'r> Criteria<'r> {
    pub fn new(req: &'r ReadRequest) -> Result<Self> {
        let criteria = Self {
            kinds: req.kinds.as_deref(),
            name: req.name_contains.as_deref().map(str::to_lowercase),
            text: req.text_contains.as_deref().map(str::to_lowercase),
            ids: req
                .ids
                .as_ref()
                .map(|ids| ids.iter().map(String::as_str).collect()),
            parent: req.parent_id.as_deref(),
        };
        if criteria.kinds.is_none()
            && criteria.name.is_none()
            && criteria.text.is_none()
            && criteria.ids.is_none()
            && criteria.parent.is_none()
        {
            return Err(ApiError::invalid(
                "find_nodes needs at least one of kinds, nameContains, textContains, ids or parentId",
            ));
        }
        Ok(criteria)
    }

    /// The criteria this node satisfies, or None when any requested criterion fails.
    pub fn matched_on(&self, node: &Node) -> Option<Vec<&'static str>> {
        let mut on = Vec::new();
        if let Some(needle) = &self.name {
            let name = node.props["name"].as_str()?;
            name.to_lowercase()
                .contains(needle.as_str())
                .then_some(())?;
            on.push("name");
        }
        if let Some(needle) = &self.text {
            let text = node.props["text"]
                .as_str()
                .filter(|_| node.kind == NodeKind::Text)?;
            text.to_lowercase()
                .contains(needle.as_str())
                .then_some(())?;
            on.push("text");
        }
        if let Some(kinds) = self.kinds {
            kinds
                .iter()
                .any(|k| k == node.kind.as_str())
                .then_some(())?;
            on.push("kind");
        }
        if let Some(ids) = &self.ids {
            ids.contains(node.id.as_str()).then_some(())?;
            on.push("id");
        }
        if let Some(parent) = self.parent {
            (node.parent_id.as_deref() == Some(parent)).then_some(())?;
            on.push("parent");
        }
        Some(on)
    }
}
