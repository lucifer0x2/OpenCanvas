//! Outer/inset CSS shadows, spread-radius correction, device-space blur and extents.
use crate::{
    blur,
    canvas::Canvas,
    color,
    geometry::{self, Rect},
    stroke, RenderError, RenderOptions,
};
use opencanvas_layout::{Paint, SceneNode};
use tiny_skia::{FillRule, PixmapPaint, Transform};

pub(crate) fn extent(n: &SceneNode) -> Option<Rect> {
    n.shadow
        .iter()
        .filter(|s| !s.inset)
        .map(|s| {
            let p = s.spread + 1.5 * s.blur + 1.0;
            let mut b = Rect::from_bounds(&n.bounds).grow(p);
            b.x += s.x;
            b.y += s.y;
            b
        })
        .filter(|b| b.w > 0.0 && b.h > 0.0)
        .reduce(Rect::union)
}
fn spread_radius(r: f32, s: f32) -> f32 {
    if r == 0.0 {
        0.0
    } else if s <= 0.0 {
        (r + s).max(0.0)
    } else if r >= s {
        r + s
    } else {
        r + s * (1.0 + (r / s - 1.0).powi(3))
    }
}
pub(crate) fn paint(
    c: &mut Canvas,
    n: &SceneNode,
    alpha: f32,
    inset: bool,
    opts: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    let bounds = Rect::from_bounds(&n.bounds);
    let radius = geometry::corners(n);
    let edges = if n.stroke.as_ref().is_some_and(|s| s.align == "inside") {
        stroke::edges(n)
    } else {
        None
    };
    let (padding, pr) = edges
        .as_ref()
        .map_or((bounds, radius), |e| (e.inner, e.inner_r));
    for s in n.shadow.iter().rev().filter(|s| s.inset == inset) {
        opts.check()?;
        if inset && (padding.w <= 0.0 || padding.h <= 0.0) {
            continue;
        }
        let source = if inset {
            Rect {
                x: padding.x + s.x + s.spread,
                y: padding.y + s.y + s.spread,
                w: padding.w - 2.0 * s.spread,
                h: padding.h - 2.0 * s.spread,
            }
        } else {
            Rect {
                x: bounds.x + s.x - s.spread,
                y: bounds.y + s.y - s.spread,
                w: bounds.w + 2.0 * s.spread,
                h: bounds.h + 2.0 * s.spread,
            }
        };
        if !inset && (source.w <= 0.0 || source.h <= 0.0) {
            continue;
        }
        let support = 1.5 * s.blur * c.scale + 3.0;
        let region = if inset {
            // The complement source extends beyond the padding box. Include its
            // blur support before clipping, otherwise inset edges incorrectly fade.
            padding.grow(1.5 * s.blur + 2.0 / c.scale)
        } else {
            source.grow(1.5 * s.blur + 2.0 / c.scale)
        };
        let Some(device) = region.device(c.scale, c.bounds().grow(support)) else {
            continue;
        };
        let w = device.w.ceil() as u32;
        let h = device.h.ceil() as u32;
        if u64::from(w) * u64::from(h) > 16_000_000 {
            return Err(RenderError::budget());
        }
        let mut scratch = Canvas::new(w, h, c.scale, device.x as i32, device.y as i32)?;
        scratch.cancelled = c.cancelled;
        let r = if inset {
            pr.map(|r| [(r[0] - s.spread).max(0.0), (r[1] - s.spread).max(0.0)])
        } else {
            radius.map(|r| [spread_radius(r[0], s.spread), spread_radius(r[1], s.spread)])
        };
        let mut mask = if let Some(p) = geometry::scan::path(
            source,
            geometry::reduced_radii(source, r),
            c.scale,
        ) {
            scratch.path_mask(&p, FillRule::Winding)?
        } else {
            tiny_skia::Mask::new(w, h).ok_or_else(RenderError::budget)?
        };
        if inset {
            mask.invert();
        }
        blur::blur(
            mask.data_mut(),
            w as usize,
            h as usize,
            s.blur * c.scale / 2.0,
            opts,
        )?;
        let opaque =
            matches!(&n.fill,Paint::Solid(s) if color::color(s).is_ok_and(|c|c.alpha()==1.0));
        if inset || !opaque {
            let cut_bounds = if inset { padding } else { bounds };
            if let Some(p) = geometry::scan::path(
                cut_bounds,
                geometry::reduced_radii(cut_bounds, if inset { pr } else { radius }),
                c.scale,
            ) {
                let mut cut = scratch.path_mask(&p, FillRule::Winding)?;
                if !inset {
                    cut.invert();
                }
                for (i, (a, b)) in mask.data_mut().iter_mut().zip(cut.data()).enumerate() {
                    if i % 4096 == 0 {
                        opts.check()?;
                    }
                    *a = ((u16::from(*a) * u16::from(*b) + 127) / 255) as u8;
                }
            }
        }
        let color = color::color(&s.color)?;
        let a = (255.0 * color.alpha() * alpha).round() as u16;
        let premul =
            [color.red(), color.green(), color.blue(), 1.0].map(|v| (v * a as f32).round() as u16);
        for (i, (pixel, m)) in scratch
            .pixmap
            .data_mut()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(mask.data())
            .enumerate()
        {
            if i % 4096 == 0 {
                opts.check()?;
            }
            // Mask coverage scales a quantized premultiplied solid; do not
            // independently round floating color * final floating alpha.
            for (dst, v) in pixel.iter_mut().zip(premul) {
                *dst = ((v * (u16::from(*m) + 1)) >> 8) as u8;
            }
        }
        c.pixmap.draw_pixmap(
            scratch.x - c.x,
            scratch.y - c.y,
            scratch.pixmap.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            c.clip.as_ref(),
        );
    }
    Ok(())
}
