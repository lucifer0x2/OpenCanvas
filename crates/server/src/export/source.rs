//! Resolve export targets from one authoritative revision, preserving custom fallback.
use super::target::{self, Nodes};
use crate::{
    error::{ApiError, Result},
    jobs::{ExportSpec, SceneRequest},
};
use opencanvas_storage::{
    read_scope::{ReadScope, ScopedSnapshot},
    Node, SharedSnapshot,
};
use serde_json::Value;

pub(super) struct Source<'a> {
    shared: &'a SharedSnapshot,
    scoped: Option<ScopedSnapshot<'a>>,
    legacy: Option<(Vec<Node>, Vec<Node>)>,
}

impl<'a> Source<'a> {
    pub fn new(shared: &'a SharedSnapshot, theme: &str) -> Result<Self> {
        let scoped = shared.trusted_scoped()?;
        let legacy = if let Some(scoped) = &scoped {
            // Keep the old global theme error before per-node missing-target errors.
            scoped.resolved_tokens(theme)?;
            None
        } else {
            let resolved = opencanvas_storage::resolved_expanded_nodes(&shared.snapshot, theme)?;
            let expanded = opencanvas_storage::instances::expand(&shared.snapshot.nodes)?;
            Some((expanded, resolved))
        };
        Ok(Self {
            shared,
            scoped,
            legacy,
        })
    }

    pub fn resolve(
        &self,
        id: &str,
        theme: &str,
        settings: Option<&Value>,
    ) -> Result<(SceneRequest, ExportSpec)> {
        if let Some(scoped) = &self.scoped {
            let mut nodes = scoped.expand(ReadScope::Target(id), false).map_err(|e| {
                if e.code == "INVALID_REQUEST" {
                    ApiError::new("NODE_NOT_FOUND", "Node does not exist at this revision")
                } else {
                    e.into()
                }
            })?;
            let resolved = scoped.resolve(&mut nodes, theme)?;
            target::resolve(
                &Nodes::new(&nodes.expanded, &resolved),
                &self.shared.snapshot,
                id,
                theme,
                settings,
            )
        } else {
            let (expanded, resolved) = self.legacy.as_ref().unwrap();
            target::resolve(
                &Nodes::new(expanded, resolved),
                &self.shared.snapshot,
                id,
                theme,
                settings,
            )
        }
    }
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
