//! One node's own paint order; descendant traversal and stroke live in layers.
use crate::{
    canvas::Canvas, color, geometry, images, placeholder, resources::Prepared, shadow, text,
    RenderError, RenderOptions,
};
use opencanvas_layout::SceneNode;

pub(crate) fn node(
    c: &mut Canvas,
    n: &SceneNode,
    r: &Prepared,
    alpha: f32,
    o: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    shadow::paint(c, n, alpha, false, o)?;
    o.check()?;
    if n.kind != "text" {
        let radii = geometry::corners(n);
        let path = if radii.iter().any(|r| r[0] > 0.0 && r[1] > 0.0) {
            geometry::scan::path(geometry::Rect::from_bounds(&n.bounds), radii, c.scale)
        } else {
            geometry::node_path(n)
        };
        if let Some(path) = path {
            color::fill(
                c,
                &path,
                &n.fill,
                geometry::Rect::from_bounds(&n.bounds),
                alpha,
                o,
            )?;
        }
    }
    if n.kind == "text" {
        if let Some(font) = n.font_id.as_ref().and_then(|f| r.fonts.get(f)) {
            text::paint(c, n, font, alpha, o)?;
        } else {
            placeholder::paint(c, n, alpha)?;
        }
    }
    if n.kind == "image" {
        if let Some(image) = n.asset_id.as_ref().and_then(|a| r.images.get(a)) {
            images::paint(c, n, image, alpha, o)?;
        } else {
            placeholder::paint(c, n, alpha)?;
        }
    }
    o.check()?;
    shadow::paint(c, n, alpha, true, o)
}
