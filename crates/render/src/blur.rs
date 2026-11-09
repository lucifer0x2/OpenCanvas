//! Three-box Gaussian approximation with sliding sums, bounded work and cancellation.
use crate::{RenderError, RenderOptions};

pub(crate) fn blur(
    data: &mut [u8],
    w: usize,
    h: usize,
    sigma: f32,
    opts: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    if sigma <= 0.0 {
        return Ok(());
    }
    // Skia's raster mask plan uses two equal windows and an odd third.
    // An even first/second window has opposite half-pixel offsets; replacing
    // these with nearby odd windows changes the entire low-alpha blur profile.
    let window = (sigma * 3.0 * (2.0 * std::f32::consts::PI).sqrt() / 4.0)
        .round()
        .max(1.0) as usize;
    let windows = [window, window, window | 1];
    let mut a: Vec<f32> = data.iter().map(|v| *v as f32).collect();
    let mut b = vec![0.0; a.len()];
    for vertical in [false, true] {
        for (pass, size) in windows.into_iter().enumerate() {
            axis(&a, &mut b, w, h, size, pass == 1, vertical, opts)?;
            std::mem::swap(&mut a, &mut b);
        }
        // The separable raster mask is A8 between axes, not a floating mask.
        for (i, v) in a.iter_mut().enumerate() {
            if i % 4096 == 0 {
                opts.check()?;
            }
            *v = v.round();
        }
    }
    for (i, (dst, v)) in data.iter_mut().zip(a).enumerate() {
        if i % 4096 == 0 {
            opts.check()?;
        }
        *dst = v.round().clamp(0.0, 255.0) as u8;
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)] // Axis dimensions/direction are explicit, not ambient state.
fn axis(
    src: &[f32],
    dst: &mut [f32],
    w: usize,
    h: usize,
    size: usize,
    reverse: bool,
    vertical: bool,
    opts: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    let (lines, len) = if vertical { (w, h) } else { (h, w) };
    let divisor = size as f32;
    let left = if reverse { size / 2 } else { (size - 1) / 2 };
    let right = size - 1 - left;
    for line in 0..lines {
        opts.check()?;
        let at = |p: usize| if vertical { p * w + line } else { line * w + p };
        let mut sum = 0.0;
        for p in 0..=right.min(len.saturating_sub(1)) {
            sum += src[at(p)];
        }
        for p in 0..len {
            dst[at(p)] = sum / divisor;
            if p >= left {
                sum -= src[at(p - left)];
            }
            if p + right + 1 < len {
                sum += src[at(p + right + 1)];
            }
        }
    }
    Ok(())
}
