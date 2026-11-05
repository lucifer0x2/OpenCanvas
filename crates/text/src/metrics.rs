use crate::{Bounds, Font, PositionedRun, TextError};

/// Canvas font bounding ascent/descent in logical pixels, independent of line contents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
}

impl Font {
    /// macOS Chrome's integer font metrics derived from CoreText's hhea line extents.
    /// Never substitutes the global head glyph bbox or a text-specific ink box.
    pub fn metrics(&self, size: f32) -> Result<FontMetrics, TextError> {
        if !size.is_finite() || !(1.0..=10000.0).contains(&size) {
            return Err(TextError::InvalidRequest);
        }
        let instance = self.shaping_font_at_size(size)?;
        let metrics = instance.metrics();
        let line = metrics.hhea_line.ok_or(TextError::FontDecode)?;
        let scale = size as f64 / self.inner.units_per_em() as f64;
        Ok(FontMetrics {
            ascent: (line.ascender.to_f64() * scale).round() as f32,
            descent: (-line.descender.to_f64() * scale).round() as f32,
        })
    }

    /// Unhinted extents of the shared paint run, with Canvas's conservative negative
    /// advance overflow bound. This never moves glyphs; empty lines have a zero box.
    pub fn ink_bounds(&self, run: &PositionedRun, size: f32) -> Result<Bounds, TextError> {
        if !size.is_finite() || !(1.0..=10000.0).contains(&size) {
            return Err(TextError::InvalidRequest);
        }
        let scale = size as f64 / self.inner.units_per_em() as f64;
        let instance = self.shaping_font_at_size(size)?;
        let metrics = instance
            .glyph_metrics()
            .with_extents_source_order(skrifa::GlyphSourceOrder::OUTLINES);
        let mut bounds: Option<(f64, f64, f64, f64)> = None;
        for glyph in &run.glyphs {
            let Some(ink) = metrics.extents(glyph.id) else {
                continue;
            };
            if ink.width == 0.0 || ink.height == 0.0 {
                continue;
            }
            let x = glyph.x + ink.x_bearing as f64 * scale;
            let y = glyph.y - ink.y_bearing as f64 * scale;
            let right = x + ink.width as f64 * scale;
            let bottom = y + ink.height as f64 * scale;
            bounds = Some(match bounds {
                Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(right), b.max(bottom)),
                None => (x, y, right, bottom),
            });
        }
        let (mut x, y, right, bottom) = bounds.unwrap_or_default();
        if bounds.is_some() && run.advance < 0.0 {
            // Blink PlainTextNode::Shape expands a negative-width run's left edge
            // to its signed advance, leaving the right edge and glyphs unchanged.
            // This is a conservative Canvas metric, not additional painted ink.
            x = x.min(run.advance);
        }
        Ok(Bounds {
            x: x as f32,
            y: y as f32,
            width: (right - x) as f32,
            height: (bottom - y) as f32,
        })
    }
}
