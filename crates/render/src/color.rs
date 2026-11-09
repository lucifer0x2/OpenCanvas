//! sRGB colors and gradient shaders with renderer-compatible premultiplied interpolation.
use crate::{canvas::Canvas, geometry::Rect, RenderError, RenderOptions};
use opencanvas_layout::Paint as ScenePaint;
use tiny_skia::{Color, FillRule, Paint, Path};
#[path = "gradient.rs"]
mod gradient;

pub(crate) fn color(s: &str) -> Result<Color, RenderError> {
    if s == "transparent" {
        return Ok(Color::TRANSPARENT);
    }
    let h = s
        .strip_prefix('#')
        .ok_or_else(|| RenderError::scene("Invalid paint color"))?;
    if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(RenderError::scene("Invalid paint color"));
    }
    let h = if h.len() == 3 || h.len() == 4 {
        h.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        h.to_owned()
    };
    if h.len() != 6 && h.len() != 8 {
        return Err(RenderError::scene("Invalid paint color"));
    }
    let byte = |i| {
        u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| RenderError::scene("Invalid paint color"))
    };
    Ok(Color::from_rgba8(
        byte(0)?,
        byte(2)?,
        byte(4)?,
        if h.len() == 8 { byte(6)? } else { 255 },
    ))
}
pub(crate) fn solid(s: &str, alpha: f32) -> Result<Paint<'static>, RenderError> {
    let mut c = color(s)?;
    c.set_alpha(c.alpha() * alpha);
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    Ok(p)
}
fn stops(stops: &[(f32, String)]) -> Result<Vec<(f32, Color)>, RenderError> {
    let mut out = Vec::new();
    for (i, (p, s)) in stops.iter().enumerate() {
        let a = color(s)?;
        if a.alpha() == 0.0 {
            let prev = if i > 0 { color(&stops[i - 1].1)? } else { a };
            let next = if let Some((_, s)) = stops.get(i + 1) {
                color(s)?
            } else {
                a
            };
            for c in [prev, next] {
                let c = Color::from_rgba(c.red(), c.green(), c.blue(), 0.0)
                    .ok_or_else(|| RenderError::scene("Invalid gradient color"))?;
                out.push((*p, c));
            }
        } else {
            out.push((*p, a));
        }
        if let Some((q, s)) = stops.get(i + 1) {
            let b = color(s)?;
            if a.alpha() == 0.0 || b.alpha() == 0.0 || a.alpha() == b.alpha() || p == q {
                continue;
            }
            for j in 1..64 {
                let t = j as f32 / 64.0;
                let alpha = a.alpha() * (1.0 - t) + b.alpha() * t;
                let mix = |x, y| (x * a.alpha() * (1.0 - t) + y * b.alpha() * t) / alpha;
                let c = Color::from_rgba(
                    mix(a.red(), b.red()),
                    mix(a.green(), b.green()),
                    mix(a.blue(), b.blue()),
                    alpha,
                )
                .ok_or_else(|| RenderError::scene("Invalid gradient"))?;
                out.push((p + (q - p) * t, c));
            }
        }
    }
    Ok(out)
}
pub(crate) fn fill(
    canvas: &mut Canvas,
    path: &Path,
    fill: &ScenePaint,
    b: Rect,
    alpha: f32,
    opts: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    if let ScenePaint::Solid(s) = fill {
        canvas.fill(path, &solid(s, alpha)?, FillRule::Winding)?;
        return Ok(());
    }
    let ScenePaint::Gradient(g) = fill else {
        unreachable!()
    };
    gradient::fill(canvas, path, g, b, alpha, opts)
}
