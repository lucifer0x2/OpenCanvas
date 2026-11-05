//! Device-space conic subdivision for Canvas general-path mask rasterization.
//!
//! Skia's general path scan converter does not integrate the exact ellipse:
//! conics become quadratics at 1/4 device-pixel tolerance, then monotone
//! quadratic edges become lines with quarter-pixel Y snapping. Keep this
//! separate from tiny-skia's midline-stroke fast path and text outlines.
//! Algorithm references: SkGeometry.cpp (computeQuadPOW2/chop) and
//! SkAnalyticEdge.cpp (diff_to_shift/updateQuadratic), google/skia.
use crate::geometry::{Radii, Rect};
use tiny_skia::{Path, PathBuilder};

type Point = [f32; 2];

pub(crate) fn path(b: Rect, r: Radii, scale: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    append(&mut pb, b, r, scale);
    pb.finish()
}

pub(crate) fn append(pb: &mut PathBuilder, b: Rect, r: Radii, scale: f32) {
    if b.w <= 0.0 || b.h <= 0.0 {
        return;
    }
    let [tl, tr, br, bl] = r;
    let (x, y, right, bottom) = (b.x, b.y, b.x + b.w, b.y + b.h);
    pb.move_to(x + tl[0], y);
    for (a, corner, z) in [
        ([right - tr[0], y], [right, y], [right, y + tr[1]]),
        ([right, bottom - br[1]], [right, bottom], [right - br[0], bottom]),
        ([x + bl[0], bottom], [x, bottom], [x, bottom - bl[1]]),
        ([x, y + tl[1]], [x, y], [x + tl[0], y]),
    ] {
        pb.line_to(a[0], a[1]);
        arc(pb, a, corner, z, scale);
    }
    pb.close();
}

pub(crate) fn arc(pb: &mut PathBuilder, a: Point, corner: Point, z: Point, scale: f32) {
    if a == corner || z == corner {
        pb.line_to(z[0], z[1]);
        return;
    }
    let points = [a, corner, z].map(|p| p.map(|v| v * scale));
    let w = std::f32::consts::FRAC_1_SQRT_2;
    let k = (w - 1.0) / (4.0 * (1.0 + w));
    let error = std::array::from_fn::<_, 2, _>(|i| {
        k * (points[0][i] - 2.0 * points[1][i] + points[2][i])
    });
    let mut error = error[0].hypot(error[1]);
    let mut depth = 0;
    while error > 0.25 && depth < 5 {
        error *= 0.25;
        depth += 1;
    }
    conic(pb, points, w, depth, scale);
}

fn conic(pb: &mut PathBuilder, p: [Point; 3], w: f32, depth: u32, scale: f32) {
    if depth == 0 {
        quadratic(pb, p, scale);
        return;
    }
    let s = 1.0 / (1.0 + w);
    let a = std::array::from_fn(|i| (p[0][i] + w * p[1][i]) * s);
    let b = std::array::from_fn(|i| (w * p[1][i] + p[2][i]) * s);
    let m = std::array::from_fn(|i| (a[i] + b[i]) * 0.5);
    let w = (0.5 + 0.5 * w).sqrt();
    conic(pb, [p[0], a, m], w, depth - 1, scale);
    conic(pb, [m, b, p[2]], w, depth - 1, scale);
}

fn quadratic(pb: &mut PathBuilder, p: [Point; 3], scale: f32) {
    // FDot6 with Skia's AA accuracy=2: truncate input at 1/256 device pixel.
    let mut p = p.map(|p| p.map(|v| (v * 256.0).trunc() / 256.0));
    let reverse = p[0][1] > p[2][1];
    if reverse {
        p.reverse();
    }
    let delta: Point = std::array::from_fn(|i| {
        ((2.0 * p[1][i] - p[0][i] - p[2][i]) * 64.0).floor().abs()
    });
    let distance = delta[0].max(delta[1]) + (delta[0].min(delta[1]) * 0.5).floor();
    let distance = ((distance + 16.0) / 32.0).floor() as u32;
    let shift = ((32 - distance.leading_zeros()) / 2).clamp(1, 6);
    let count = 1 << shift;
    let snap = |v: f32| (v * 4.0 + 0.5).floor() / 4.0;
    let mut last = [p[0][0], snap(p[0][1])];
    let mut previous = p[0];
    let mut edges = Vec::with_capacity(count + 1);
    edges.push(last);
    for step in 1..=count {
        let t = step as f32 / count as f32;
        let next: Point = std::array::from_fn(|i| {
            (p[0][i] - 2.0 * p[1][i] + p[2][i]) * t * t
                + 2.0 * (p[1][i] - p[0][i]) * t
                + p[0][i]
        });
        let dy = next[1] - previous[1];
        let dx = next[0] - previous[0];
        let mut edge = [next[0], snap(next[1]).min(snap(p[2][1]))];
        if step < count && dy.abs() >= 2.0 && dy.abs() * 64.0 > dx.abs() {
            let slope = (next[0] - last[0]) / (next[1] - last[1]);
            edge[1] = (next[1] + 0.5).floor().min(snap(p[2][1]));
            edge[0] -= slope * (next[1] - edge[1]);
        }
        edges.push(edge);
        last = edge;
        previous = next;
    }
    if reverse {
        edges.reverse();
    }
    for p in edges {
        pb.line_to(p[0] / scale, p[1] / scale);
    }
}
