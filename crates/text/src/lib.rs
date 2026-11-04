//! Deterministic, caller-owned font bytes; no filesystem, system fonts, threads or runtime.
//! Measurement and painting share [`Font::positioned_run`]. See the crate README for limits.
mod coverage;
mod decode;
mod error;
mod font;
mod font_set;
mod itemize;
mod measure;
mod metrics;
mod optical;
mod shaping;
mod transform;
mod wrap;

pub use error::TextError;
pub use font::{Face, Font};
pub use font_set::FontSet;
pub use metrics::FontMetrics;
pub use opencanvas_layout::{Bounds, MeasureRequest, Measurement, TextLine};
pub use shaping::{PositionedGlyph, PositionedRun};
pub use transform::transform_text;
pub use wrap::wrap_text;
// Re-export the exact outline API/version used by shaping for renderer integration.
pub use skrifa::{DrawMemory, GlyphId, PathSink, ScalerFont};

/// Authored text limit, matching the storage validator (UTF-8 bytes).
pub const MAX_TEXT_BYTES: usize = 64 * 1024;
/// Unicode full casing can expand one scalar to three scalars.
pub const MAX_RUN_BYTES: usize = MAX_TEXT_BYTES * 3;
