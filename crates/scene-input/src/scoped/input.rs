//! Scene preparation shares the same frame contract and budgets as full input.
use super::{ReadScope, ScopedSnapshot};
use crate::{check_viewport, frame_input, ResolvedNodes, Result, SceneOptions, StorageError};
use serde_json::Value;

impl ScopedSnapshot<'_> {
    pub fn make_input_with_scale(&self, options: &SceneOptions, scale: f64) -> Result<Value> {
        check_viewport(options.width, options.height, scale)?;
        // Requested theme errors precede frame lookup, as on the full path.
        crate::validate::check_scoped_theme(self.snapshot, &options.theme)?;
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
        frame_input(self.snapshot, &resolved, options)
    }
}
