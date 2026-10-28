//! Synchronous Taffy layout with external batched browser measurement.
mod engine;
mod model;
#[cfg(test)]
mod tests;
mod tree;
mod util;
mod validation;
#[cfg(test)]
mod wrap_tests;
pub use engine::compute;
pub use model::*;
pub const SCENE_VERSION: &str = "scene-1";
pub const RENDERER_VERSION: &str = "web-1";
pub const MAX_PASSES: usize = 6;
pub const MAX_NODES: usize = 2048;
pub const MAX_PIXELS: f32 = 16_000_000.0;
