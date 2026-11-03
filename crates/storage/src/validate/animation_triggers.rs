//! Hover sources are authored tree references, checked on the final batch candidate.
use super::Candidate;
use crate::{error::*, model::*};
use serde_json::Value;

impl Candidate {
    pub(super) fn validate_animation_triggers(&self) -> Result<()> {
        for target in self.nodes.values() {
            let Some(animations) = target.props.get("animations").and_then(Value::as_array) else {
                continue;
            };
            for (i, animation) in animations.iter().enumerate() {
                // Literal shape and on:"hover" are checked with the other animation keys.
                let Some(source) = animation.get("triggerNodeId").and_then(Value::as_str) else {
                    continue;
                };
                let mut ancestors = self.ancestors(&target.id);
                let valid = source != target.id
                    && ancestors
                        .find(|node| node.id == source)
                        .is_some_and(|node| {
                            matches!(node.kind, NodeKind::Group | NodeKind::Frame)
                                && (self.is_root_frame(source)
                                    || ancestors.any(|node| self.is_root_frame(&node.id)))
                        });
                if !valid {
                    return Err(StorageError::validation(
                        &format!("props.animations[{i}].triggerNodeId"),
                        "triggerNodeId must identify an authored group or frame that is a strict ancestor in the same root frame",
                    )
                    .on_node(&target.id, target.kind.as_str()));
                }
            }
        }
        Ok(())
    }
}
