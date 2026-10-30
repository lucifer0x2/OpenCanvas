//! Path and paint helpers shared by the painter modules: canvas `roundRect`, node outline, solid
//! paint, and tiny-skia's mask product (private in the crate, copied so results stay identical).
use crate::scene::{parse_color, Node};
use tiny_skia::{Color, Paint, Path, PathBuilder};

const K: f32 = 0.552_284_8;

/// Canvas roundRect with per-corner elliptical radii [tl, tr, br, bl] as (rx, ry).
pub fn round_rect(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, r: [(f32, f32); 4]) {
    let mut r = r;
    let f = [w / (r[0].0 + r[1].0), w / (r[3].0 + r[2].0), h / (r[0].1 + r[3].1), h / (r[1].1 + r[2].1)]
        .iter().fold(1f32, |m, v| if v.is_finite() { m.min(*v) } else { m });
    if f < 1.0 { for c in r.iter_mut() { c.0 *= f; c.1 *= f; } }
    let [tl, tr, br, bl] = r;
    pb.move_to(x + tl.0, y);
    pb.line_to(x + w - tr.0, y);
    pb.cubic_to(x + w - tr.0 * (1.0 - K), y, x + w, y + tr.1 * (1.0 - K), x + w, y + tr.1);
    pb.line_to(x + w, y + h - br.1);
    pb.cubic_to(x + w, y + h - br.1 * (1.0 - K), x + w - br.0 * (1.0 - K), y + h, x + w - br.0, y + h);
    pb.line_to(x + bl.0, y + h);
    pb.cubic_to(x + bl.0 * (1.0 - K), y + h, x, y + h - bl.1 * (1.0 - K), x, y + h - bl.1);
    pb.line_to(x, y + tl.1);
    pb.cubic_to(x, y + tl.1 * (1.0 - K), x + tl.0 * (1.0 - K), y, x + tl.0, y);
    pb.close();
}

/// The node's bounds as a rounded rect (scene units).
pub fn node_path(n: &Node) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let b = n.bounds;
    round_rect(&mut pb, b.x, b.y, b.width, b.height, [(n.radius, n.radius); 4]);
    pb.finish()
}

/// Anti-aliased solid paint of a scene color at `alpha`; None for "transparent".
pub fn solid(color: &str, alpha: f32) -> Option<Paint<'static>> {
    let c = parse_color(color)?;
    let mut p = Paint::default();
    p.set_color(Color::from_rgba(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0 * alpha)?);
    p.anti_alias = true;
    Some(p)
}

/// `tiny_skia::color::premultiply_u8`, the product `Mask::intersect_path` applies per byte.
pub fn mask_product(a: u8, b: u8) -> u8 {
    let prod = u32::from(a) * u32::from(b) + 128;
    ((prod + (prod >> 8)) >> 8) as u8
}
