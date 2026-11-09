//! Per-frame content digests: the browser cache identity, independent of the project revision.
use crate::{SceneOptions, Snapshot, StorageError};
use opencanvas_scene_input::frame_content_bases;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, StorageError>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameKeyRequest {
    pub frame_id: String,
    pub width: f64,
    pub height: f64,
}

/// A digest, or the frame's own error; the caller computes such frames without a cache.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameKey {
    pub frame_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<StorageError>,
}

/// One snapshot parse and one resolution pass for every requested frame.
pub fn frame_keys(
    snapshot: &Snapshot,
    frames: &[FrameKeyRequest],
    theme: &str,
) -> Result<Vec<FrameKey>> {
    let options: Vec<_> = frames
        .iter()
        .map(|frame| SceneOptions {
            frame_id: frame.frame_id.clone(),
            width: frame.width,
            height: frame.height,
            theme: theme.into(),
            state: "default".into(),
        })
        .collect();
    let bases = frame_content_bases(snapshot, theme, &options)?;
    frames
        .iter()
        .zip(bases)
        .map(|(frame, basis)| {
            let (digest, error) = match basis {
                Ok(basis) => (Some(digest(&serde_json::to_vec(&basis)?)), None),
                Err(error) => (None, Some(error)),
            };
            Ok(FrameKey {
                frame_id: frame.frame_id.clone(),
                digest,
                error,
            })
        })
        .collect()
}

pub fn frame_keys_json(snapshot_json: &str, frames_json: &str, theme: &str) -> Result<String> {
    let snapshot: Snapshot = serde_json::from_str(snapshot_json)?;
    let frames: Vec<FrameKeyRequest> = serde_json::from_str(frames_json)?;
    Ok(serde_json::to_string(&frame_keys(
        &snapshot, &frames, theme,
    )?)?)
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
