//! Reusable font resolution against one already-expanded immutable revision.
use super::{scope::Resolver, Candidate};
use crate::{Node, NodeKind, Result, Snapshot};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "font_references_tests.rs"]
mod tests;

pub struct FontReferences<'a> {
    candidate: Cow<'a, Candidate>,
    expanded: &'a BTreeMap<String, Node>,
    themes: BTreeSet<String>,
}

impl<'a> FontReferences<'a> {
    pub fn from_candidate(
        candidate: &'a Candidate,
        expanded: &'a BTreeMap<String, Node>,
    ) -> Result<Self> {
        Ok(Self {
            candidate: Cow::Borrowed(candidate),
            expanded,
            themes: candidate.validate_tokens()?,
        })
    }

    /// Only resource maps are copied; authored and expanded nodes stay borrowed.
    pub fn from_snapshot(
        snapshot: &Snapshot,
        expanded: &'a BTreeMap<String, Node>,
    ) -> Result<Self> {
        let candidate = Candidate {
            theme: "default".into(),
            nodes: BTreeMap::new(),
            tokens: snapshot
                .tokens
                .iter()
                .cloned()
                .map(|v| (v.id.clone(), v))
                .collect(),
            styles: snapshot
                .styles
                .iter()
                .cloned()
                .map(|v| (v.id.clone(), v))
                .collect(),
            fonts: snapshot
                .fonts
                .iter()
                .cloned()
                .map(|v| (v.id.clone(), v))
                .collect(),
            assets: BTreeMap::new(),
            notes: BTreeMap::new(),
            deleted: BTreeSet::new(),
        };
        let themes = candidate.validate_tokens()?;
        Ok(Self {
            candidate: Cow::Owned(candidate),
            expanded,
            themes,
        })
    }

    /// Retains authored references and placed overrides under every token theme.
    pub fn for_nodes(&self, nodes: &[&Node]) -> Result<BTreeSet<String>> {
        let mut ids = BTreeSet::new();
        for theme in &self.themes {
            let resolver = Resolver {
                candidate: &self.candidate,
                nodes: self.expanded,
                theme,
            };
            for node in nodes.iter().filter(|n| n.kind == NodeKind::Text) {
                if let Some(id) = resolver
                    .resolve_scoped_props(node)?
                    .get("fontId")
                    .and_then(|v| v.as_str())
                {
                    ids.insert(id.into());
                }
            }
        }
        Ok(ids)
    }
}

/// Compatibility helper for callers without an existing expansion.
pub fn referenced_font_ids(snapshot: &Snapshot, nodes: &[&Node]) -> Result<BTreeSet<String>> {
    let expanded = crate::instances::expand(&snapshot.nodes)?
        .into_iter()
        .map(|n| (n.id.clone(), n))
        .collect();
    FontReferences::from_snapshot(snapshot, &expanded)?.for_nodes(nodes)
}
