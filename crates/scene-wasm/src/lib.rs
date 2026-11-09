//! Snapshot → resolved scene. No storage, network, paint, clock or browser APIs.
#[cfg(target_arch = "wasm32")]
mod bindings;
mod engine;

pub use engine::{ResourceUrls, SceneEngine};
pub use opencanvas_scene_input::{SceneOptions, Snapshot, StorageError};
