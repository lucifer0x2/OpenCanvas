//! CSS border geometry, including elliptical inner radii and fast midline coverage.
use crate::{
    canvas::Canvas,
    color,
    geometry::{self, Radii, Rect},
    RenderError,
};
use opencanvas_layout::SceneNode;
use tiny_skia::{FillRule, PathBuilder, Stroke};


pub(crate) struct Edges {
    pub outer: Rect,
    pub inner: Rect,
    pub outer_r: Radii,
    pub inner_r: Radii,
    pub k: [[f32; 2]; 4],
}
pub(crate) fn edges(n: &SceneNode) -> Option<Edges> {
    let s = n.stroke.as_ref()?;
    if !s.width.iter().any(|w| *w > 0.0) {
        return None;
    }
    let [t, r, b, l] = s.width;
    let f = match s.align.as_str() {
        "outside" => 1.0,
        "center" => 0.5,
        _ => 0.0,
    };
    let k = [[l, t], [r, t], [r, b], [l, b]];
    let radii = geometry::corners(n);
    let bounds = Rect::from_bounds(&n.bounds);
    let outer = Rect {
        x: bounds.x - l * f,
        y: bounds.y - t * f,
        w: bounds.w + (l + r) * f,
        h: bounds.h + (t + b) * f,
    };
    let outer_r = std::array::from_fn(|i| {
        if radii[i][0] > 0.0 {
            [radii[i][0] + k[i][0] * f, radii[i][1] + k[i][1] * f]
        } else {
            [0.0; 2]
        }
    });
    let inner = Rect {
        x: outer.x + l,
        y: outer.y + t,
        w: outer.w - l - r,
        h: outer.h - t - b,
    };
    let inner_r = std::array::from_fn(|i| {
        [
            (outer_r[i][0] - k[i][0]).max(0.0),
            (outer_r[i][1] - k[i][1]).max(0.0),
        ]
    });
    (outer.w > 0.0 && outer.h > 0.0).then_some(Edges {
        outer,
        inner,
        outer_r,
        inner_r,
        k,
    })
}
fn fits(e: &Edges) -> bool {
    let r = e.inner_r;
    let i = e.inner;
    r[0][0] + r[1][0] <= i.w
        && r[3][0] + r[2][0] <= i.w
        && r[0][1] + r[3][1] <= i.h
        && r[1][1] + r[2][1] <= i.h
}
fn midline(e: &Edges) -> f32 {
    let w = e.k[0][1];
    if w > 0.0
        && e.k.iter().all(|r| r[0] == w && r[1] == w)
        && e.outer.w > w
        && e.outer.h > w
        && e.outer_r
            .iter()
            .all(|r| (r[0] == 0.0 && r[1] == 0.0) || (r[0] > w / 2.0 && r[1] > w / 2.0))
    {
        w
    } else {
        0.0
    }
}
/// Nonzero union of outside-inner-box and each corner's outside-ellipse piece.
/// Unlike roundRect this does not reduce overlapping inner ellipses a second time.
fn outside_inner(c: &mut Canvas, e: &Edges) -> Result<(), RenderError> {
    let mut p = PathBuilder::new();
    geometry::append(&mut p, e.outer.grow(1.0), [[0.0; 2]; 4]);
    let i = e.inner;
    let right = i.x + i.w;
    let bottom = i.y + i.h;
    p.move_to(right, i.y);
    p.line_to(i.x, i.y);
    p.line_to(i.x, bottom);
    p.line_to(right, bottom);
    p.close();
    let k = 0.552_284_8;
    for (n, [rx, ry]) in e.inner_r.into_iter().enumerate() {
        if rx <= 0.0 || ry <= 0.0 {
            continue;
        }
        let origin = i.corners()[n];
        let sx = if n == 0 || n == 3 { 1.0 } else { -1.0 };
        let sy = if n < 2 { 1.0 } else { -1.0 };
        let point = |x: f32, y: f32| (origin[0] + sx * x, origin[1] + sy * y);
        let (a, b) = point(rx - rx * k, 0.0);
        let (d, f) = point(0.0, ry - ry * k);
        let (x, y) = point(0.0, ry);
        p.move_to(origin[0], origin[1]);
        if sx * sy > 0.0 {
            let (right, top) = point(rx, 0.0);
            p.line_to(right, top);
            p.cubic_to(a, b, d, f, x, y);
        } else {
            // Reverse this piece to keep winding aligned with the outer path.
            p.line_to(x, y);
            let (x, y) = point(rx, 0.0);
            p.cubic_to(d, f, a, b, x, y);
        }
        p.close();
    }
    if let Some(p) = p.finish() {
        c.intersect_clip(&p)?;
    }
    Ok(())
}
pub(crate) fn ring(
    c: &mut Canvas,
    e: &Edges,
    s: &str,
    alpha: f32,
    single: bool,
) -> Result<(), RenderError> {
    let paint = color::solid(s, alpha)?;
    let w = if single { midline(e) } else { 0.0 };
    if w > 0.0 {
        let h = w / 2.0;
        let r = e
            .outer_r
            .map(|r| [(r[0] - h).max(0.0), (r[1] - h).max(0.0)]);
        if let Some(p) = geometry::path(e.outer.grow(-h), r) {
            let t = c.transform();
            c.pixmap.stroke_path(
                &p,
                &paint,
                &Stroke {
                    width: w,
                    ..Stroke::default()
                },
                t,
                c.clip.as_ref(),
            );
        }
    } else if e.inner.w > 0.0 && e.inner.h > 0.0 && !fits(e) {
        let old = c.clip.clone();
        outside_inner(c, e)?;
        if let Some(p) = geometry::path(e.outer, e.outer_r) {
            c.fill(&p, &paint, FillRule::Winding)?;
        }
        c.clip = old;
    } else {
        let mut p = PathBuilder::new();
        geometry::scan::append(&mut p, e.outer, e.outer_r, c.scale);
        geometry::scan::append(&mut p, e.inner, e.inner_r, c.scale);
        if let Some(p) = p.finish() {
            c.fill(&p, &paint, FillRule::EvenOdd)?;
        }
    }
    Ok(())
}
pub(crate) fn paint(c: &mut Canvas, n: &SceneNode, alpha: f32) -> Result<(), RenderError> {
    let Some(e) = edges(n) else {
        return Ok(());
    };
    let s = n.stroke.as_ref().expect("edges require stroke");
    let colors: Vec<_> = (0..4)
        .filter(|i| s.width[*i] > 0.0)
        .map(|i| &s.color[i])
        .collect();
    if colors.iter().all(|v| *v == colors[0]) {
        ring(c, &e, colors[0], alpha, true)
    } else {
        crate::stroke_sides::paint(c, &e, s, alpha)
    }
}
