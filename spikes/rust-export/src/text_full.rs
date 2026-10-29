//! The original Part D text paint (`--paint full`), kept unchanged as the reference `paint-gate`
//! compares the bbox painter against: per text node a whole-canvas coverage mask, a whole-canvas
//! clip multiply and a whole-canvas `fill_rect` through the mask.
use crate::font::Shaper;
use crate::profile::PaintProfile;
use crate::scene::Node;
use crate::shape::solid;
use crate::text::{TextOpts, TextPainter};
use std::time::Instant;
use tiny_skia::{Mask, PathBuilder, Pixmap, Rect, Transform};

fn ms(t: Instant) -> f64 { t.elapsed().as_secs_f64() * 1e3 }

/// Paints one text node; `area` and timings go to `p` (raster = coverage, composite = clip + fill).
#[allow(clippy::too_many_arguments)]
pub fn paint_text(text: &TextPainter, shaper: &Shaper, opts: TextOpts, px: &mut Pixmap, n: &Node, a: f32, clip: Option<&Mask>, k: f32, p: &mut PaintProfile) {
    let t = Instant::now();
    let mut pb = PathBuilder::new();
    for l in &n.lines {
        text.add_line(shaper, &mut pb, &l.text, n.bounds.x + l.x, n.bounds.y + l.y, n.font_size, k, opts);
    }
    p.text_nodes += 1;
    p.text_area += (n.bounds.width * n.bounds.height * k * k) as f64;
    p.path_ms += ms(t);
    let t = Instant::now();
    let Some(mut mask) = text.coverage(pb, px.width(), px.height(), opts) else { return };
    p.mask_area += (px.width() * px.height()) as f64;
    p.raster_ms += ms(t);
    let t = Instant::now();
    if let Some(c) = clip {
        for (m, c) in mask.data_mut().iter_mut().zip(c.data()) { *m = ((*m as u32 * *c as u32 + 127) / 255) as u8; }
    }
    let Some(paint) = solid(&n.fill, a) else { return };
    let r = Rect::from_xywh(0.0, 0.0, px.width() as f32, px.height() as f32).unwrap();
    px.fill_rect(r, &paint, Transform::identity(), Some(&mask));
    p.composite_ms += ms(t);
}
