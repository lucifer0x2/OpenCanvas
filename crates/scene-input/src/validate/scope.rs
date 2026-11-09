//! Borrowed resolution context: node scopes never clone the candidate maps.
use super::Candidate;
use crate::{Node, Result, StorageError};
use serde_json::Value;
use std::ops::Deref;

pub(super) struct Resolver<'a> {
    pub candidate: &'a Candidate,
    pub theme: &'a str,
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
        }
    }
    pub(super) fn effective_theme<'a>(&'a self, node: &'a Node) -> Result<&'a str> {
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
        Ok(&self.theme)
    }
    pub fn resolve_props(&self, node: &Node) -> Result<Value> {
        Resolver {
            candidate: self,
            theme: self.effective_theme(node)?,
        }
        .resolve_props(node)
    }
    pub fn resolve_value(&self, value: &Value, expected: &str) -> Result<Value> {
        self.resolver().resolve_value(value, expected)
    }
    pub(super) fn resolve_paint(&self, value: &Value) -> Result<Value> {
        self.resolver().resolve_paint(value)
    }
}
