//! Where paint time goes (Part D), per render. Timings are wall time except `path_ms`.
use serde::Serialize;

#[derive(Default, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaintProfile {
    /// Pool size the render ran with (1 without the `parallel` feature).
    pub threads: u32,
    pub text_nodes: u32,
    /// Device px of text node bounds (what the old D8 estimate scaled by).
    pub text_area: f64,
    /// Device px of the coverage masks: the whole canvas per node (full) or glyph bounds (bbox).
    pub mask_area: f64,
    /// Bbox masks rasterized on a canvas-size mask instead (text_mask.rs `coverage`).
    pub fallbacks: u32,
    /// Glyph path building, summed over text nodes (CPU time; wall time at 1 thread).
    pub path_ms: f64,
    /// Full: coverage + gamma LUT per node. Bbox: the parallel pre-pass (paths, coverage, LUT).
    pub raster_ms: f64,
    /// Clip multiply + fill of every text node, in paint order.
    pub composite_ms: f64,
    /// Node fills, images and strokes (tiny-skia `fill_path`; includes first-touch page faults
    /// of the fresh canvas).
    pub shape_ms: f64,
    /// Child clip masks: full = a new mask per clipping node and a copy per other parent;
    /// bbox = a new mask per clipping node only.
    pub masks: u32,
    pub mask_ms: f64,
}
