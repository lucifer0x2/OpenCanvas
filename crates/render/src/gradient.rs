//! Straight-stop interpolation followed by premultiplication and ordered RGB
//! dithering, as in Canvas's sRGB RGBA8 raster pipeline. Alpha is never dithered.
use crate::{canvas::Canvas, geometry::Rect, RenderError, RenderOptions};
use opencanvas_layout::{Gradient, Radius};
use tiny_skia::{Color, Path};

enum Coordinates {
    Linear { x: f32, y: f32, dx: f32, dy: f32 },
    Radial { x: f32, y: f32, rx: f32, ry: f32 },
}
impl Coordinates {
    fn at(&self, x: f32, y: f32) -> f32 {
        match *self {
            Self::Linear {
                x: cx,
                y: cy,
                dx,
                dy,
            } => (x - cx) * dx + (y - cy) * dy + 0.5,
            Self::Radial {
                x: cx,
                y: cy,
                rx,
                ry,
            } => (((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2)).sqrt(),
        }
    }
}
fn interpolate(stops: &[(f32, Color)], t: f32) -> [f32; 4] {
    // The last coincident stop owns the right side of a hard transition.
    let right = stops.partition_point(|(p, _)| *p <= t);
    let (a, b, k) = if right == 0 {
        (stops[0].1, stops[0].1, 0.0)
    } else if right == stops.len() {
        (stops[right - 1].1, stops[right - 1].1, 0.0)
    } else {
        let (p, a) = stops[right - 1];
        let (q, b) = stops[right];
        (a, b, (t - p) / (q - p))
    };
    let a = [a.red(), a.green(), a.blue(), a.alpha()];
    let b = [b.red(), b.green(), b.blue(), b.alpha()];
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * k)
}
fn dither(x: i32, y: i32) -> f32 {
    // Bayer 8x8: interleave the low three bits of x and x XOR y.
    let z = x ^ y;
    let m = ((z & 1) << 5)
        | ((x & 1) << 4)
        | ((z & 2) << 2)
        | ((x & 2) << 1)
        | ((z & 4) >> 1)
        | ((x & 4) >> 2);
    (m as f32 + 0.5) / 64.0 - 0.5
}
pub(super) fn fill(
    canvas: &mut Canvas,
    path: &Path,
    g: &Gradient,
    b: Rect,
    alpha: f32,
    opts: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    let (coords, source) = match g {
        Gradient::Linear { angle, stops } => {
            let a = angle.to_radians();
            let (dx, dy) = (a.sin(), -a.cos());
            let len = (b.w * dx).abs() + (b.h * dy).abs();
            (
                Coordinates::Linear {
                    x: b.x + b.w / 2.0,
                    y: b.y + b.h / 2.0,
                    dx: dx / len,
                    dy: dy / len,
                },
                stops,
            )
        }
        Gradient::Radial {
            center: [cx, cy],
            radius,
            stops,
        } => {
            let [rx, ry] = match radius {
                Some(Radius::Circle(r)) => [*r; 2],
                Some(Radius::Ellipse(r)) => *r,
                None => [
                    2.0_f32.sqrt() * (cx * b.w).abs().max((b.w - cx * b.w).abs()),
                    2.0_f32.sqrt() * (cy * b.h).abs().max((b.h - cy * b.h).abs()),
                ],
            };
            (
                Coordinates::Radial {
                    x: b.x + cx * b.w,
                    y: b.y + cy * b.h,
                    rx,
                    ry,
                },
                stops,
            )
        }
    };
    let stops = super::stops(source)?;
    let (scale, ox, oy) = (canvas.scale, canvas.x, canvas.y);
    canvas.shade(path, b, true, opts, |x, y| {
        let c = interpolate(&stops, coords.at((x + 0.5) / scale, (y + 0.5) / scale));
        let a = c[3] * alpha;
        let noise = dither(x as i32 - ox, y as i32 - oy) / 255.0;
        let mut out = [0; 4];
        for i in 0..3 {
            out[i] = ((c[i] * a + noise).clamp(0.0, a) * 255.0).round() as u8;
        }
        out[3] = (a * 255.0).round() as u8;
        out
    })
}
