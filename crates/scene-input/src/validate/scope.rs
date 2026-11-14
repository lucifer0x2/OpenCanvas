//! Borrowed resolution context: node scopes never clone the candidate maps.
use super::Candidate;
use crate::{Node, Result, StorageError};
use serde_json::Value;
use std::collections::BTreeMap;
use std::ops::Deref;

pub(super) struct Resolver<'a> {
    pub candidate: &'a Candidate,
    pub theme: &'a str,
    pub nodes: &'a BTreeMap<String, Node>,
}
impl Deref for Resolver<'_> {
    type Target = Candidate;
    fn deref(&self) -> &Candidate {
        self.candidate
    }
}
impl Candidate {
    pub(super) fn resolver(&self) -> Resolver<'_> {
        Resolver {
            candidate: self,
            theme: &self.theme,
            nodes: &self.nodes,
        }
    }
    pub(super) fn effective_theme<'a>(&'a self, node: &'a Node) -> Result<&'a str> {
        self.resolver().effective_theme(node)
    }
    pub fn resolve_props(&self, node: &Node) -> Result<Value> {
        self.resolver().resolve_scoped_props(node)
    }
    pub fn resolve_value(&self, value: &Value, expected: &str) -> Result<Value> {
        self.resolver().resolve_value(value, expected)
    }
    pub(super) fn node(&self, node: &Node) -> Result<()> {
        self.resolver().node(node)
    }
    pub(super) fn style(&self, style: &crate::Style) -> Result<()> {
        self.resolver().style(style)
    }
}
impl<'context> Resolver<'context> {
    pub(super) fn effective_theme<'a>(&self, node: &'a Node) -> Result<&'a str>
    where
        'context: 'a,
    {
        let mut current = Some(node);
        // Expansion can concatenate several individually valid authored ancestor chains.
        for _ in 0..=self.nodes.len() {
            let Some(n) = current else { break };
            if let Some(value) = n.props.get("theme") {
                let name = value.as_str().ok_or_else(|| {
                    StorageError::validation("props.theme", "Theme must be a string")
                })?;
                if name != "default"
                    && !self.tokens.values().any(|t| {
                        t.value
                            .get("themes")
                            .and_then(Value::as_object)
                            .is_some_and(|themes| themes.contains_key(name))
                    })
                {
                    let mut error = StorageError::new(
                        "UNKNOWN_THEME",
                        "Scoped theme is not declared by any token",
                    );
                    error.field = Some("props.theme".into());
                    return Err(error);
                }
                return Ok(name);
            }
            current = n.parent_id.as_ref().and_then(|id| self.nodes.get(id));
        }
        Ok(self.theme)
    }
    pub(super) fn resolve_scoped_props(&self, node: &Node) -> Result<Value> {
        Resolver {
            candidate: self,
            theme: self.effective_theme(node)?,
            nodes: self.nodes,
        }
        .resolve_props(node)
    }
    pub(super) fn ancestors(&self, id: &str) -> impl Iterator<Item = &Node> {
        let parent = |node: &Node| node.parent_id.as_ref().and_then(|id| self.nodes.get(id));
        std::iter::successors(self.nodes.get(id).and_then(parent), move |node| {
            parent(node)
        })
        .take(128)
    }
    pub(super) fn is_root_frame(&self, id: &str) -> bool {
        self.nodes.get(id).is_some_and(|n| {
            n.kind == crate::NodeKind::Frame
                && n.parent_id
                    .as_ref()
                    .and_then(|id| self.nodes.get(id))
                    .is_some_and(|n| n.kind == crate::NodeKind::Page)
        })
    }
}
