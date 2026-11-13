//! Scene preparation shares the same frame contract and budgets as full input.
use super::{ReadScope, ScopedNodes, ScopedSnapshot};
use crate::{check_viewport, frame_input, ResolvedNodes, Result, SceneOptions, StorageError};
use serde_json::Value;

impl<'a> ScopedSnapshot<'a> {
    pub fn make_input_with_scale(&self, options: &SceneOptions, scale: f64) -> Result<Value> {
        self.prepare_input_with_scale(options, scale)
            .map(|(input, _)| input)
    }

    /// Prepare exact scene bytes and retain their authored dependency closure for
    /// indexed note intent. The caller can drop the closure before queue admission.
    pub fn prepare_input_with_scale(
        &self,
        options: &SceneOptions,
        scale: f64,
    ) -> Result<(Value, ScopedNodes<'a>)> {
        check_viewport(options.width, options.height, scale)?;
        // Requested theme errors precede frame lookup, as on the full path.
        self.check_theme(&options.theme)?;
        let mut nodes = self
            .expand(ReadScope::Target(&options.frame_id), false)
            .map_err(|error| {
                if error.code == "INVALID_REQUEST" {
                    StorageError::new("INVALID_REQUEST", "frameId must identify a frame")
                } else {
                    error
                }
            })?;
        let resolved = ResolvedNodes::from_nodes(self.resolve(&mut nodes, &options.theme)?);
        Ok((frame_input(self.snapshot, &resolved, options)?, nodes))
    }
}
