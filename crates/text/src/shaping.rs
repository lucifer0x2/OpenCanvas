use crate::{Font, TextError, MAX_RUN_BYTES};
use harfrust::{Buffer, Direction, Feature, Language, ShapeOptions, ShaperFont, Tag};
use unicode_segmentation::UnicodeSegmentation;

/// Pixel coordinates relative to the alphabetic origin; positive y points DOWN.
#[derive(Clone, Debug)]
pub struct PositionedGlyph {
    pub id: skrifa::GlyphId,
    /// UTF-8 byte offset in the supplied (already transformed) text.
    pub cluster: u32,
    pub x: f64,
    pub y: f64,
    /// Pen movement INCLUDING this cluster's tracking, identical in paint/measurement.
    pub x_advance: f64,
    pub y_advance: f64,
}

#[derive(Clone, Debug, Default)]
pub struct PositionedRun {
    pub glyphs: Vec<PositionedGlyph>,
    /// Signed Canvas advance (float accumulation of script-item widths). It may
    /// differ from an f64 sum of glyph advances. Measurement clamps then ceil64s.
    pub advance: f64,
}

impl Font {
    /// The single shaping/positioning implementation for both wrapping and painting.
    /// Kerning stays enabled; nonzero tracking disables optional ligatures, as in Blink.
    /// This is an LTR run, not paragraph bidi itemization.
    pub fn positioned_run(
        &self,
        text: &str,
        size: f32,
        spacing: f32,
    ) -> Result<PositionedRun, TextError> {
        if text.len() > MAX_RUN_BYTES {
            return Err(TextError::TextLimit);
        }
        if !size.is_finite()
            || !(1.0..=10000.0).contains(&size)
            || !spacing.is_finite()
            || !(-1000.0..=1000.0).contains(&spacing)
        {
            return Err(TextError::InvalidRequest);
        }
        let instance = self.shaping_font_at_size(size)?;
        let size = size as f64;
        let scale = (size * 65536.0) as i32;
        let scaled = ShaperFont::new(&instance).with_scale(scale);
        let glyph_metrics = instance.glyph_metrics();
        let features = [
            Feature::new(Tag::new(b"liga"), 0, ..),
            Feature::new(Tag::new(b"clig"), 0, ..),
        ];
        let mut run = PositionedRun {
            glyphs: Vec::new(),
            advance: 0.0,
        };
        let mut pen_y = 0.0;
        for item in crate::itemize::items(text) {
            let origin = run.advance;
            // Script items, unlike wrap opportunities, may start inside a grapheme.
            let starts: Vec<usize> = text[item.range.clone()]
                .grapheme_indices(true)
                .filter(|(_, g)| g.chars().any(|c| !crate::coverage::default_ignorable(c)))
                .map(|(at, _)| item.range.start + at)
                .collect();
            let mut buffer = Buffer::new();
            // Preserve original UTF-8 cluster offsets through Canvas normalization.
            for (offset, c) in text[item.range.clone()].char_indices() {
                let c = if matches!(c, '\t' | '\n' | '\r' | '\u{c}' | '\u{2028}' | '\u{2029}') {
                    ' '
                } else {
                    c
                };
                buffer.push(c as u32, (item.range.start + offset) as u32);
            }
            buffer.set_direction(Direction::LeftToRight);
            buffer.set_language(Language::new("und"));
            buffer.set_script(Some(item.script));
            let options = if spacing == 0.0 {
                ShapeOptions::new()
            } else {
                ShapeOptions::new().features(&features)
            };
            harfrust::shape(&scaled, &mut buffer, options).map_err(|_| TextError::Shape)?;
            let infos = buffer.glyph_infos();
            let positions = buffer.glyph_positions();
            for (i, (info, pos)) in infos.iter().zip(positions).enumerate() {
                let id = skrifa::GlyphId::new(info.glyph_id);
                // Blink's nominal advance passes through float then truncates to 16.16.
                // Preserve HarfRust's SCALED GPOS adjustment instead of rounding design units.
                // Keep fractional HVAR advances; an unscaled HarfBuzz integer advance
                // would discard variation deltas before the pixel-scale conversion.
                let nominal = glyph_metrics.h_advance(id) as f64;
                let px = (nominal * size / self.inner.units_per_em() as f64) as f32;
                let blink = (px * 65536.0) as i64;
                // HarfBuzz zeroes default-ignorables AFTER shaping. Do not introduce a tiny
                // nominal rounding delta back into that deliberately zero-width glyph.
                let hidden = pos.x_advance == 0
                    && text[info.cluster as usize..]
                        .chars()
                        .next()
                        .is_some_and(crate::coverage::default_ignorable);
                let advance = if hidden {
                    0
                } else {
                    pos.x_advance as i64 - scaled.glyph_h_advance(id) as i64 + blink
                };
                let cluster_end = infos
                    .get(i + 1)
                    .map_or(item.range.end, |next| next.cluster as usize);
                let tracking = if cluster_end > info.cluster as usize {
                    let lo = starts.partition_point(|at| *at < info.cluster as usize);
                    let hi = starts.partition_point(|at| *at < cluster_end);
                    (hi - lo) as f64 * spacing as f64
                } else {
                    0.0
                };
                let x_advance = advance as f64 / 65536.0 + tracking;
                let y_advance = -(pos.y_advance as f64) / 65536.0;
                run.glyphs.push(PositionedGlyph {
                    id,
                    cluster: info.cluster,
                    x: run.advance + pos.x_offset as f64 / 65536.0,
                    y: pen_y - pos.y_offset as f64 / 65536.0,
                    x_advance,
                    y_advance,
                });
                run.advance += x_advance;
                pen_y += y_advance;
            }
            // Blink stores script-run widths and their accumulated width as float.
            // Keep the fixed-point glyph advances, but preserve that operation order.
            run.advance = ((origin as f32) + ((run.advance - origin) as f32)) as f64;
        }
        Ok(run)
    }
}
