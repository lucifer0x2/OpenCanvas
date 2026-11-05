//! Bounded tiled subpixel coverage for the software path rasterizer.
//! tiny-skia's legacy four-bit edge coverage is too coarse for Canvas clips;
//! integrate a 4x raster into A8 without supersampling the color/image shader.
use crate::RenderError;
use tiny_skia::{FillRule, Mask, Path, Transform};

pub(super) fn raster(
    path: &Path,
    rule: FillRule,
    transform: Transform,
    width: u32,
    height: u32,
    cancelled: Option<&dyn Fn() -> bool>,
) -> Result<Mask, RenderError> {
    let mut result = Mask::new(width, height).ok_or_else(RenderError::budget)?;
    let bounds = path
        .bounds()
        .transform(transform)
        .ok_or_else(|| RenderError::scene("Invalid path transform"))?;
    let left = bounds.left().floor().max(0.0) as u32;
    let top = bounds.top().floor().max(0.0) as u32;
    let right = bounds.right().ceil().max(0.0).min(width as f32) as u32;
    let bottom = bounds.bottom().ceil().max(0.0).min(height as f32) as u32;
    const TILE: u32 = 128;
    const SUB: u32 = 4;
    for y in (top..bottom).step_by(TILE as usize) {
        for x in (left..right).step_by(TILE as usize) {
            if cancelled.is_some_and(|f| f()) {
                return Err(RenderError::new("CANCELLED", "Render cancelled by caller"));
            }
            let w = (right - x).min(TILE);
            let h = (bottom - y).min(TILE);
            let mut fine = Mask::new(w * SUB, h * SUB).ok_or_else(RenderError::budget)?;
            let t = transform
                .post_translate(-(x as f32), -(y as f32))
                .post_scale(SUB as f32, SUB as f32);
            fine.fill_path(path, rule, true, t);
            for dy in 0..h {
                for dx in 0..w {
                    let mut sum = 0u32;
                    for sy in 0..SUB {
                        let row = ((dy * SUB + sy) * w * SUB + dx * SUB) as usize;
                        sum += fine.data()[row..row + SUB as usize]
                            .iter()
                            .map(|v| u32::from(*v))
                            .sum::<u32>();
                    }
                    result.data_mut()[((y + dy) * width + x + dx) as usize] =
                        ((sum + SUB * SUB / 2) / (SUB * SUB)) as u8;
                }
            }
        }
    }
    Ok(result)
}
