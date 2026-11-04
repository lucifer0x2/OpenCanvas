//! The spike painter as a library (Task 09): scene subset, HarfRust shaping, skrifa outlines and the
//! tiny-skia painter. No file I/O and no threads without the `parallel` feature, so it builds for
//! wasm32-unknown-unknown; font and image arrive as bytes. The CLI (main.rs) re-exports these modules.
pub mod clip;
pub mod clock;
pub mod font;
pub mod mode;
pub mod paint;
pub mod par;
pub mod profile;
pub mod scene;
pub mod shape;
pub mod text;
pub mod text_full;
pub mod text_mask;
