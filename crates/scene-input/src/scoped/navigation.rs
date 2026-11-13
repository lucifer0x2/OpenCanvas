//! Navigation metadata for the expanded read view.
use super::ScopedSnapshot;
use crate::{Node, Result};

impl ScopedSnapshot<'_> {
    /// Validated component subtrees cannot carry links/flowStart/animations,
    /// and instance roots/overrides cannot add links. Therefore a committed
    /// expanded view has exactly the same incoming sources as its authored view.
    /// Preserve legacy full expansion for untrusted/custom backend read paths.
    pub fn navigation_expanded_nodes(&self, nodes: &[Node], frame: &str) -> Result<Vec<Node>> {
        let borrowed: Vec<_> = nodes.iter().collect();
        Ok(self
            .navigation_nodes(&borrowed, frame)
            .into_iter()
            .cloned()
            .collect())
    }
}
