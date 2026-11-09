//! Snapshot → resolved scene. No storage, network, paint, clock or browser APIs.
#[cfg(target_arch = "wasm32")]
mod bindings;
mod content_key;
mod engine;

pub use content_key::{frame_keys, frame_keys_json, FrameKey, FrameKeyRequest};
pub use engine::{ResourceUrls, SceneEngine};
pub use opencanvas_scene_input::{SceneOptions, Snapshot, StorageError};
