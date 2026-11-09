//! Glyph outlines positioned ONLY by opencanvas-text's shared Blink-quantized run.
use crate::{
    canvas::Canvas,
    color,
    geometry::{self, Rect},
    RenderError, RenderOptions,
};
use opencanvas_layout::SceneNode;
use opencanvas_text::{DrawMemory, Font, PathSink, ScalerFont};
use tiny_skia::{FillRule, PathBuilder, Transform};

struct Sink<'a> {
    pb: &'a mut PathBuilder,
    x: f32,
    y: f32,
    scale: f32,
}
impl PathSink for Sink<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.pb
            .move_to(self.x + x * self.scale, self.y - y * self.scale);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.pb
            .line_to(self.x + x * self.scale, self.y - y * self.scale);
    }
    fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
        self.pb.quad_to(
            self.x + a * self.scale,
            self.y - b * self.scale,
            self.x + x * self.scale,
            self.y - y * self.scale,
        );
    }
    fn curve_to(&mut self, a: f32, b: f32, d: f32, e: f32, x: f32, y: f32) {
        self.pb.cubic_to(
            self.x + a * self.scale,
            self.y - b * self.scale,
            self.x + d * self.scale,
            self.y - e * self.scale,
            self.x + x * self.scale,
            self.y - y * self.scale,
        );
    }
    fn close(&mut self) {
        self.pb.close();
    }
}
pub(crate) fn paint(
    c: &mut Canvas,
    n: &SceneNode,
    font: &Font,
    alpha: f32,
    o: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    let paint = color::solid(n.fill.as_solid(), alpha)?;
    let instance = font
        .shaping_font_at_size(n.font_size)
        .map_err(|_| RenderError::new("TEXT_SHAPING_FAILED", "Could not instance scene font"))?;
    let scaler = ScalerFont::unhinted(instance.clone(), None);
    let scale = n.font_size / instance.units_per_em() as f32;
    for l in &n.lines {
        o.check()?;
        let run = font
            .positioned_run(&l.text, n.font_size, n.letter_spacing)
            .map_err(|_| {
                RenderError::new("TEXT_SHAPING_FAILED", "Could not position scene line")
            })?;
        let mut pb = PathBuilder::new();
        for glyph in run.glyphs {
            o.check()?;
            let mut x = n.bounds.x + l.x + glyph.x as f32;
            let mut y = n.bounds.y + l.y + glyph.y as f32;
            if o.snap_glyph_origins {
                x = (x * c.scale * 4.0).round() / (c.scale * 4.0);
                y = (y * c.scale).round() / c.scale;
            }
            if let Some(outline) = scaler.outline(glyph.id) {
                outline
                    .draw(
                        DrawMemory::Scratch,
                        &mut Sink {
                            pb: &mut pb,
                            x,
                            y,
                            scale,
                        },
                    )
                    .map_err(|_| {
                        RenderError::new("UNSUPPORTED_FEATURE", "Glyph outline could not be drawn")
                    })?;
            } else if instance
                .glyph_metrics()
                .extents(glyph.id)
                .is_some_and(|e| e.width != 0.0 && e.height != 0.0)
            {
                return Err(RenderError::new(
                    "UNSUPPORTED_FEATURE",
                    "Non-outline glyph rasterization is not supported",
                ));
            }
        }
        if let Some(path) = pb.finish() {
            if o.text_coverage_gamma == 1.0 {
                c.fill(&path, &paint, FillRule::Winding)?;
            } else {
                let mut mask = c.path_mask(&path, FillRule::Winding)?;
                for (i, pixel) in mask.data_mut().iter_mut().enumerate() {
                    if i % 4096 == 0 {
                        o.check()?;
                    }
                    *pixel = ((*pixel as f32 / 255.0).powf(1.0 / o.text_coverage_gamma) * 255.0)
                        .round() as u8;
                }
                if let Some(clip) = &c.clip {
                    for (a, b) in mask.data_mut().iter_mut().zip(clip.data()) {
                        *a = ((u16::from(*a) * u16::from(*b) + 127) / 255) as u8;
                    }
                }
                let rect = tiny_skia::Rect::from_xywh(
                    0.0,
                    0.0,
                    c.pixmap.width() as f32,
                    c.pixmap.height() as f32,
                )
                .ok_or_else(RenderError::budget)?;
                c.pixmap
                    .fill_rect(rect, &paint, Transform::identity(), Some(&mask));
            }
        }
        if let Some(d) = &n.decoration {
            // JS Math.round ties toward positive infinity, including negative origins.
            let b = Rect {
                x: n.bounds.x + l.x,
                y: (n.bounds.y + l.y + d.offset + 0.5).floor(),
                w: l.width,
                h: d.thickness,
            };
            if let Some(p) = geometry::path(b, [[0.0; 2]; 4]) {
                c.fill(&p, &paint, FillRule::Winding)?;
            }
        }
    }
    Ok(())
}
