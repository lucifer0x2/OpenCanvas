//! Pure scene-3/web-4 rasterization. No filesystem, URL loader, runtime, or threads.
mod api;
mod blur;
mod canvas;
mod color;
mod encode;
mod geometry;
mod icc;
mod images;
mod layers;
mod paint;
mod placeholder;
mod resources;
mod shadow;
mod stroke;
mod stroke_sides;
mod svg_resources;
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
mod text;
mod tree;
mod validation;
mod wire;

pub use api::{DrawStats, PaintScene, Raster, RenderError, RenderOptions, ResourceReadiness};
pub use encode::encode_png;
pub use resources::{ResourceBytes, Resources};
pub use svg_resources::inspect_svg_resources;

/// Decode/inspect strict resources without allocating a target (e.g. an SVG export).
/// No geometry is painted; observations on a strict failure live in RenderError.readiness.
pub fn inspect_resources(
    scene: &PaintScene,
    resources: &Resources<'_>,
    options: &RenderOptions<'_>,
) -> Result<Vec<ResourceReadiness>, RenderError> {
    validation::resource_identities(scene, resources, options)?;
    Ok(resources.prepare(scene, options)?.readiness)
}

/// Renders an already-computed scene; never measures, wraps, or transforms its lines.
pub fn render(
    scene: &PaintScene,
    resources: &Resources<'_>,
    options: &RenderOptions<'_>,
) -> Result<Raster, RenderError> {
    let (width, height) = validation::validate(scene, resources, options)?;
    options.check()?;
    let prepared = resources.prepare(scene, options)?;
    let tree = tree::Tree::new(scene, &prepared)?;
    let mut canvas = canvas::Canvas::new(width, height, options.scale as f32, 0, 0)?;
    canvas.cancelled = options.cancelled;
    let mut stats = DrawStats {
        drawn_lines: scene.nodes.iter().map(|n| n.lines.len()).sum(),
        ..DrawStats::default()
    };
    layers::paint(&mut canvas, scene, &tree, &prepared, options, &mut stats)?;
    Ok(Raster {
        width,
        height,
        rgba: canvas.pixmap.take(),
        draw: stats,
        source_scene_key: scene.provenance.source_scene_key.clone(),
        diagnostics: prepared.diagnostics,
        readiness: prepared.readiness,
    })
}
