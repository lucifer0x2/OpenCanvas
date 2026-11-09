//! Revision-free identity of everything one frame scene is computed from.
use crate::{
    check_viewport, frame_input, scene_diagnostics, Node, ResolvedNodes, Result, SceneOptions,
    Snapshot, StorageError,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// Bumped whenever the basis below gains or loses an input.
pub const CONTENT_BASIS_VERSION: u32 = 1;

/// The exact layout input (subtree, resolved tokens/styles/themes, expanded instances and
/// overrides, used font faces and asset hashes, viewport/theme/state) plus the note-derived
/// overflow intent, without the global revision. Equal bases compute equal scenes, apart from
/// provenance revision, under one build.
pub fn frame_content_basis(
    snapshot: &Snapshot,
    resolved: &ResolvedNodes,
    options: &SceneOptions,
) -> Result<Value> {
    check_viewport(options.width, options.height, 1.0)?;
    let mut input = frame_input(snapshot, resolved, options)?;
    let intentional = if snapshot.notes.is_empty() {
        Vec::new()
    } else {
        let nodes: Vec<Node> = Deserialize::deserialize(&input["nodes"])?;
        scene_diagnostics::overflow_ids(snapshot, &nodes)
    };
    if let Some(fields) = input.as_object_mut() {
        fields.remove("revision");
    }
    Ok(json!({
        "basis": CONTENT_BASIS_VERSION,
        "input": input,
        "intentionalOverflow": intentional,
    }))
}

/// One resolution pass for many frames of one theme. A snapshot-level failure fails all;
/// a frame-level failure (unknown frame, budget) stays with that frame.
pub fn frame_content_bases(
    snapshot: &Snapshot,
    theme: &str,
    frames: &[SceneOptions],
) -> Result<Vec<Result<Value>>> {
    let resolved = ResolvedNodes::new(snapshot, theme)?;
    Ok(frames
        .iter()
        .map(|options| {
            if options.theme != theme {
                return Err(StorageError::new(
                    "INVALID_REQUEST",
                    "Batched frames must share the resolved theme",
                ));
            }
            frame_content_basis(snapshot, &resolved, options)
        })
        .collect())
}
