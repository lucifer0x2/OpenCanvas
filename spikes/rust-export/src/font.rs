//! Font loading and HarfRust shaping: advances in px and positioned glyph runs.
use harfrust::{Buffer, Direction, Font, Language, Script, ShapeOptions, ShaperFont, ShapePlan};
use std::str::FromStr;

pub const FONT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../renderer/fonts/NotoSans.ttf");

pub struct Shaper {
    pub font: Font,
    pub upem: f32,
    plan: ShapePlan,
}

/// One positioned glyph, in px relative to the line origin (y down).
pub struct PlacedGlyph {
    pub id: harfrust::GlyphId,
    pub x: f32,
    pub y: f32,
}

impl Shaper {
    pub fn load() -> Shaper {
        let bytes = std::fs::read(FONT_PATH).expect("font");
        let font = Font::new(bytes, 0).expect("parse font");
        let upem = font.units_per_em() as f32;
        let lang = Language::from_str("vi").unwrap();
        let plan = {
            ShapePlan::new(&font, Direction::LeftToRight, Some(Script::LATIN), Some(&lang), &[])
        };
        Shaper { font, upem, plan }
    }

    fn shape(&self, text: &str) -> Buffer {
        let sf = ShaperFont::new(&self.font);
        let mut buf = Buffer::new();
        buf.push_str(text);
        buf.set_direction(Direction::LeftToRight);
        buf.set_script(Some(Script::LATIN));
        buf.set_language(Some(Language::from_str("vi").unwrap()));
        harfrust::shape(&sf, &mut buf, ShapeOptions::new().plan(Some(&self.plan))).expect("shape");
        buf
    }

    /// Sum of shaped x advances, scaled to `size` px (unrounded, f64).
    pub fn width(&self, text: &str, size: f64) -> f64 {
        let buf = self.shape(text);
        let units: i64 = buf.glyph_positions().iter().map(|p| p.x_advance as i64).sum();
        units as f64 * size / self.upem as f64
    }

    /// Positioned glyphs for drawing at `size` px.
    pub fn glyphs(&self, text: &str, size: f32) -> Vec<PlacedGlyph> {
        let buf = self.shape(text);
        let s = size / self.upem;
        let mut pen = 0f32;
        let mut out = Vec::new();
        for (info, pos) in buf.glyph_infos().iter().zip(buf.glyph_positions()) {
            out.push(PlacedGlyph {
                id: harfrust::GlyphId::new(info.glyph_id),
                x: pen + pos.x_offset as f32 * s,
                y: -(pos.y_offset as f32) * s,
            });
            pen += pos.x_advance as f32 * s;
        }
        out
    }
}

impl Shaper {
    /// Chrome/Blink-style width model: HarfBuzz positions in 16.16 fixed at `size`.
    /// variant 1: HarfRust scales advances and GPOS itself (scale = size·65536).
    /// variant 2: like Blink — each nominal advance comes from a float px advance truncated to
    ///            16.16 (SkScalarToHarfBuzzPosition), GPOS deltas from HarfRust's scaled shaping.
    pub fn width_fixed(&self, text: &str, size: f64, variant: u8) -> f64 {
        let scale = (size * 65536.0) as i32;
        let sf = ShaperFont::new(&self.font).with_scale(scale);
        let unscaled = ShaperFont::new(&self.font);
        let mut buf = Buffer::new();
        buf.push_str(text);
        buf.set_direction(Direction::LeftToRight);
        buf.set_script(Some(Script::LATIN));
        buf.set_language(Some(Language::from_str("vi").unwrap()));
        harfrust::shape(&sf, &mut buf, ShapeOptions::new().plan(Some(&self.plan))).expect("shape");
        let mut sum: i64 = 0;
        let mut fsum: f32 = 0.0;
        for (info, pos) in buf.glyph_infos().iter().zip(buf.glyph_positions()) {
            let gid = harfrust::GlyphId::new(info.glyph_id);
            let adv = pos.x_advance as i64;
            let one = match variant {
                1 => adv,
                _ => {
                    let units = unscaled.glyph_h_advance(gid) as f64;
                    let px = (units * size / self.upem as f64) as f32;
                    let blink = (px * 65536.0) as i64; // truncation toward zero
                    adv - sf.glyph_h_advance(gid) as i64 + blink
                }
            };
            sum += one;
            fsum += one as f32 / 65536.0;
        }
        if variant == 3 { fsum as f64 } else { sum as f64 / 65536.0 }
    }
}
