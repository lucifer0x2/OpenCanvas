//! Bbox text paint (`--paint bbox`, the default): each text node's glyph path is rasterized into a
//! mask the size of its pixel bounds (+1 px), so the gamma LUT, the clip multiply and the fill touch
//! only those pixels instead of the whole canvas (text_full.rs). Coverage does not depend on the
//! pixmap, so all visible text nodes are rasterized up front in parallel (par.rs) and composited
//! later, in paint order. Output is byte-identical to text_full.rs; `paint-gate` checks it.
use crate::font::Shaper;
use crate::profile::PaintProfile;
use crate::par;
use crate::scene::{Node, Scene, Tree};
use crate::text::{gamma_lut, TextOpts, TextPainter};
use std::sync::OnceLock;
use std::time::Instant;
use tiny_skia::{FillRule, Mask, Paint, Path, PathBuilder, PathSegment, Pixmap, Rect, Transform};

/// Margin around the glyph bounds; tiny-skia's AA rasterizer writes only inside floor/ceil of them.
const MARGIN: i64 = 1;
/// tiny-skia tiles (and drops AA past) 8191 px; beyond that only the full-canvas raster is exact.
const MAX_EXACT: u32 = 8191;

/// Gamma-mapped coverage of one text node over device px [x, x + w) × [y, y + h), row-major.
pub struct TextMask { pub x: u32, pub y: u32, pub w: u32, pub h: u32, pub data: Vec<u8> }

struct Raster { mask: Option<TextMask>, path_ms: f64, fallback: bool }

/// `RUST_SPIKE_FORCE_FALLBACK` set: every node takes the canvas-size fallback, so `paint-gate` can
/// check that path too (the Part D frames never need it).
fn forced_fallback() -> bool {
    static FORCED: OnceLock<bool> = OnceLock::new();
    *FORCED.get_or_init(|| std::env::var_os("RUST_SPIKE_FORCE_FALLBACK").is_some())
}

fn ms(t: Instant) -> f64 { t.elapsed().as_secs_f64() * 1e3 }

/// Rasterizes every visible text node in parallel; the result is indexed by node.
#[allow(clippy::too_many_arguments)]
pub fn raster_all(text: &TextPainter, shaper: &Shaper, opts: TextOpts, s: &Scene, t: &Tree, k: f32, canvas: (u32, u32), p: &mut PaintProfile) -> Vec<Option<TextMask>> {
    let started = Instant::now();
    let mut ids = Vec::new();
    for &r in &t.roots { visible_text(s, t, r, 1.0, &mut ids); }
    let rasters = par::map(&ids, |&i| raster(text, shaper, opts, &s.nodes[i], k, canvas));
    let mut out: Vec<Option<TextMask>> = (0..s.nodes.len()).map(|_| None).collect();
    for (&i, r) in ids.iter().zip(rasters) {
        let b = s.nodes[i].bounds;
        p.text_nodes += 1;
        p.text_area += (b.width * b.height * k * k) as f64;
        p.path_ms += r.path_ms;
        p.fallbacks += r.fallback as u32;
        p.mask_area += r.mask.as_ref().map_or(0.0, |m| (m.w * m.h) as f64);
        out[i] = r.mask;
    }
    p.raster_ms += ms(started);
    out
}

/// Text nodes `Painter::draw` reaches with alpha > 0, in paint order (same alpha walk).
fn visible_text(s: &Scene, t: &Tree, i: usize, alpha: f32, out: &mut Vec<usize>) {
    let a = alpha * s.nodes[i].opacity;
    if a <= 0.0 { return; }
    let a = if a < 1.0 && t.units[i] >= 2 { 1.0 } else { a };
    if s.nodes[i].kind == "text" { out.push(i); }
    for &c in t.children.get(&i).into_iter().flatten() { visible_text(s, t, c, a, out); }
}

fn raster(text: &TextPainter, shaper: &Shaper, opts: TextOpts, n: &Node, k: f32, canvas: (u32, u32)) -> Raster {
    let t = Instant::now();
    let mut pb = PathBuilder::new();
    for l in &n.lines {
        text.add_line(shaper, &mut pb, &l.text, n.bounds.x + l.x, n.bounds.y + l.y, n.font_size, k, opts);
    }
    let path = pb.finish();
    let path_ms = ms(t);
    let Some(path) = path else { return Raster { mask: None, path_ms, fallback: false } };
    let (mut mask, fallback) = coverage(&path, canvas);
    if let (Some(m), Some(lut)) = (mask.as_mut(), gamma_lut(opts)) {
        for v in &mut m.data { *v = lut[*v as usize]; }
    }
    Raster { mask, path_ms, fallback }
}

/// Coverage of `path` over its pixel bounds, clipped to the canvas. The path is shifted by whole
/// pixels into a bounds-size mask when that is provably exact (see `y_monotonic`); otherwise it is
/// rasterized exactly like the full painter, on a canvas-size mask, and cut to the bounds (`true`).
fn coverage(path: &Path, (cw, ch): (u32, u32)) -> (Option<TextMask>, bool) {
    let b = path.bounds();
    let (l, t) = (b.left().floor() as i64 - MARGIN, b.top().floor() as i64 - MARGIN);
    let (r, bo) = (b.right().ceil() as i64 + MARGIN, b.bottom().ceil() as i64 + MARGIN);
    let (x0, y0, x1, y1) = (l.max(0), t.max(0), r.min(cw as i64), bo.min(ch as i64));
    if x0 >= x1 || y0 >= y1 { return (None, false); }
    let (x, y, w, h) = (x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32);
    let inside = (x0, y0, x1, y1) == (l, t, r, bo) && cw <= MAX_EXACT && ch <= MAX_EXACT;
    if inside && y_monotonic(path) && !forced_fallback() {
        let mut m = Mask::new(w, h).unwrap();
        m.fill_path(path, FillRule::Winding, true, Transform::from_translate(-(x as f32), -(y as f32)));
        return (Some(TextMask { x, y, w, h, data: m.take() }), false);
    }
    let mut full = Mask::new(cw, ch).unwrap();
    full.fill_path(path, FillRule::Winding, true, Transform::identity());
    let rows = full.data().chunks_exact(cw as usize).skip(y as usize).take(h as usize);
    let data = rows.flat_map(|row| &row[x as usize..(x + w) as usize]).copied().collect();
    (Some(TextMask { x, y, w, h, data }), true)
}

/// Whether shifting the path by whole pixels leaves its raster exactly shifted. Points stay exact
/// (x − n is representable for 0 ≤ n ≤ x) and tiny-skia turns them into fixed point by truncation,
/// so lines and y-monotonic quads give shifted edges. A quad with a y extremum inside it is chopped
/// in f32 first (`chop_quad_at_y_extrema`), and that rounding depends on magnitude: not exact.
/// Cubics (CFF fonts) are not analysed and always take the exact fallback.
fn y_monotonic(path: &Path) -> bool {
    let (mut start, mut last) = (0f32, 0f32);
    path.segments().all(|seg| match seg {
        PathSegment::MoveTo(p) => { (start, last) = (p.y, p.y); true }
        PathSegment::LineTo(p) => { last = p.y; true }
        PathSegment::QuadTo(c, p) => { let ok = !chops(last, c.y, p.y); last = p.y; ok }
        PathSegment::CubicTo(..) => false,
        PathSegment::Close => { last = start; true }
    })
}

/// tiny-skia's `is_not_monotonic` test minus its `a == b` case, which never chops.
fn chops(a: f32, b: f32, c: f32) -> bool {
    let ab = a - b;
    let mut bc = b - c;
    if ab < 0.0 { bc = -bc; }
    ab != 0.0 && bc < 0.0
}

/// Clip multiply (the full painter's rounding) and fill of one mask, in paint order. `scratch` is a
/// zeroed canvas-size mask (tiny-skia wants mask size = pixmap size): the bounds are copied in,
/// filled through with `fill_rect` over the bounds only, then zeroed again.
pub fn composite(px: &mut Pixmap, scratch: &mut Mask, m: &TextMask, paint: &Paint, clip: Option<&Mask>) {
    let (cw, x, w) = (px.width() as usize, m.x as usize, m.w as usize);
    let at = |row: usize| (m.y as usize + row) * cw + x;
    let data = scratch.data_mut();
    for (row, src) in m.data.chunks_exact(w).enumerate() {
        let dst = &mut data[at(row)..at(row) + w];
        match clip {
            Some(c) => {
                for ((d, s), c) in dst.iter_mut().zip(src).zip(&c.data()[at(row)..at(row) + w]) {
                    *d = ((*s as u32 * *c as u32 + 127) / 255) as u8;
                }
            }
            None => dst.copy_from_slice(src),
        }
    }
    let r = Rect::from_xywh(m.x as f32, m.y as f32, m.w as f32, m.h as f32).unwrap();
    px.fill_rect(r, paint, Transform::identity(), Some(scratch));
    let data = scratch.data_mut();
    for row in 0..m.h as usize { data[at(row)..at(row) + w].fill(0); }
}
