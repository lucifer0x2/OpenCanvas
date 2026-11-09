//! Taffy layout with batched cached or synchronous text measurement.
mod engine;
mod measurement;
mod model;
#[cfg(test)]
mod paint_tests;
mod responsive;
#[cfg(test)]
mod responsive_tests;
mod scene_builder;
mod stack;
#[cfg(test)]
mod stack_tests;
#[cfg(test)]
mod stretch_tests;
#[cfg(test)]
mod stroke_tests;
#[cfg(test)]
mod tests;
mod text_lines;
mod tree;
mod util;
mod validation;
#[cfg(test)]
mod wrap_tests;
pub use engine::{compute, compute_with_measurer};
pub use measurement::TextMeasurer;
pub use model::*;
pub use responsive::{MAX_BREAKPOINTS, percentage, valid_breakpoint_width};
pub const SCENE_VERSION: &str = "scene-3";
pub const RENDERER_VERSION: &str = "web-4";
pub const MAX_PASSES: usize = 6;
pub const MAX_NODES: usize = 2048;
pub const MAX_PIXELS: f32 = 16_000_000.0;
/// Scene values of image `fit`: the vocabulary the storage validator accepts.
pub const IMAGE_FITS: [&str; 3] = ["fit", "contain", "cover"];
/// Missing means `contain`; `fill`, the scene name of `fit` before they were unified,
/// still arrives from older layout inputs and becomes `fit`.
pub(crate) fn scene_fit(fit: Option<&str>) -> Option<&'static str> {
    match fit.unwrap_or("contain") {
        "fill" => Some("fit"),
        fit => IMAGE_FITS.into_iter().find(|v| *v == fit),
    }
}
