//! Text as filled glyph outlines (skrifa, unhinted) positioned by HarfRust, with optional
//! coverage gamma and Skia-like glyph origin snapping, for parity experiments.
use crate::font::Shaper;
use skrifa::{DrawMemory, PathSink, ScalerFont};
use tiny_skia::{FillRule, Mask, PathBuilder, Transform};

#[derive(Clone, Copy)]
pub struct TextOpts {
    /// Coverage exponent: mask = cov^(1/gamma). 1.0 = linear coverage.
    pub gamma: f32,
    /// Snap glyph origins: x to 1/4 device px, baseline y to whole device px.
    pub snap: bool,
}

struct Sink<'a> { pb: &'a mut PathBuilder, ox: f32, oy: f32, s: f32 }
impl PathSink for Sink<'_> {
    fn move_to(&mut self, x: f32, y: f32) { self.pb.move_to(self.ox + x * self.s, self.oy - y * self.s) }
    fn line_to(&mut self, x: f32, y: f32) { self.pb.line_to(self.ox + x * self.s, self.oy - y * self.s) }
    fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
        self.pb.quad_to(self.ox + a * self.s, self.oy - b * self.s, self.ox + x * self.s, self.oy - y * self.s)
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        let (ox, oy, s) = (self.ox, self.oy, self.s);
        self.pb.cubic_to(ox + a * s, oy - b * s, ox + c * s, oy - d * s, ox + x * s, oy - y * s)
    }
    fn close(&mut self) { self.pb.close() }
}

pub struct TextPainter { scaler: ScalerFont }

impl TextPainter {
    pub fn new(shaper: &Shaper) -> Self {
        TextPainter { scaler: ScalerFont::unhinted(shaper.font.clone(), None) }
    }

    /// Adds one line's glyphs to `mask` (device px). `x`, `baseline` are scene units.
    #[allow(clippy::too_many_arguments)]
    pub fn add_line(&self, shaper: &Shaper, pb: &mut PathBuilder, text: &str, x: f32, baseline: f32, size: f32, scale: f32, opts: TextOpts) {
        let s = size * scale / shaper.upem;
        for g in shaper.glyphs(text, size) {
            let mut ox = (x + g.x) * scale;
            let mut oy = (baseline + g.y) * scale;
            if opts.snap { ox = (ox * 4.0).round() / 4.0; oy = oy.round(); }
            if let Some(outline) = self.scaler.outline(g.id) {
                let mut sink = Sink { pb, ox, oy, s };
                let _ = outline.draw(DrawMemory::Scratch, &mut sink);
            }
        }
    }

    /// Whole-canvas coverage mask of `pb` with the gamma tweak applied (full painter, text_full.rs).
    pub fn coverage(&self, pb: PathBuilder, w: u32, h: u32, opts: TextOpts) -> Option<Mask> {
        let path = pb.finish()?;
        let mut mask = Mask::new(w, h)?;
        mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
        if let Some(lut) = gamma_lut(opts) {
            for p in mask.data_mut() { *p = lut[*p as usize]; }
        }
        Some(mask)
    }
}

/// Coverage → mask byte for `opts.gamma`; None when gamma is 1 (linear, no pass at all).
pub fn gamma_lut(opts: TextOpts) -> Option<[u8; 256]> {
    if (opts.gamma - 1.0).abs() <= 1e-6 { return None; }
    Some(std::array::from_fn(|v| ((v as f32 / 255.0).powf(1.0 / opts.gamma) * 255.0).round() as u8))
}
