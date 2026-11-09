//! CSS corner overlap reduction and elliptical rounded-rectangle paths.
use opencanvas_layout::{Bounds, SceneNode};
use tiny_skia::{Path, PathBuilder};

#[path = "path_scan.rs"]
pub(crate) mod scan;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
pub(crate) type Radii = [[f32; 2]; 4];
impl Rect {
    pub fn from_bounds(b: &Bounds) -> Self {
        Self {
            x: b.x,
            y: b.y,
            w: b.width,
            h: b.height,
        }
    }
    pub fn grow(self, p: f32) -> Self {
        Self {
            x: self.x - p,
            y: self.y - p,
            w: self.w + 2.0 * p,
            h: self.h + 2.0 * p,
        }
    }
    pub fn intersect(self, b: Self) -> Option<Self> {
        let x = self.x.max(b.x);
        let y = self.y.max(b.y);
        let w = (self.x + self.w).min(b.x + b.w) - x;
        let h = (self.y + self.h).min(b.y + b.h) - y;
        (w > 0.0 && h > 0.0).then_some(Self { x, y, w, h })
    }
    pub fn union(self, b: Self) -> Self {
        let x = self.x.min(b.x);
        let y = self.y.min(b.y);
        Self {
            x,
            y,
            w: (self.x + self.w).max(b.x + b.w) - x,
            h: (self.y + self.h).max(b.y + b.h) - y,
        }
    }
    pub fn device(self, scale: f32, clip: Self) -> Option<Self> {
        if ![self.x, self.y, self.w, self.h]
            .iter()
            .all(|n| n.is_finite())
        {
            return Some(clip);
        }
        Self {
            x: (self.x * scale).floor() - 1.0,
            y: (self.y * scale).floor() - 1.0,
            w: (self.x * scale + self.w * scale).ceil() - (self.x * scale).floor() + 2.0,
            h: (self.y * scale + self.h * scale).ceil() - (self.y * scale).floor() + 2.0,
        }
        .intersect(clip)
    }
    pub fn corners(self) -> [[f32; 2]; 4] {
        [
            [self.x, self.y],
            [self.x + self.w, self.y],
            [self.x + self.w, self.y + self.h],
            [self.x, self.y + self.h],
        ]
    }
}
pub(crate) fn corners(node: &SceneNode) -> Radii {
    let b = &node.bounds;
    let r = node.radius.each();
    let mut f = 1.0_f32;
    for (side, sum) in [
        (b.width, r[0] + r[1]),
        (b.width, r[2] + r[3]),
        (b.height, r[0] + r[3]),
        (b.height, r[1] + r[2]),
    ] {
        if sum > 0.0 {
            f = f.min(side / sum);
        }
    }
    r.map(|r| [r * f.max(0.0); 2])
}
/// `reduce=false` is important for the inner radii of asymmetric borders.
pub(crate) fn append(pb: &mut PathBuilder, b: Rect, r: Radii) {
    if b.w <= 0.0 || b.h <= 0.0 {
        return;
    }
    let [a, c, d, e] = r;
    let x = b.x;
    let y = b.y;
    let right = x + b.w;
    let bottom = y + b.h;
    let k = 0.552_284_8;
    pb.move_to(x + a[0], y);
    pb.line_to(right - c[0], y);
    pb.cubic_to(
        right - c[0] + c[0] * k,
        y,
        right,
        y + c[1] - c[1] * k,
        right,
        y + c[1],
    );
    pb.line_to(right, bottom - d[1]);
    pb.cubic_to(
        right,
        bottom - d[1] + d[1] * k,
        right - d[0] + d[0] * k,
        bottom,
        right - d[0],
        bottom,
    );
    pb.line_to(x + e[0], bottom);
    pb.cubic_to(
        x + e[0] - e[0] * k,
        bottom,
        x,
        bottom - e[1] + e[1] * k,
        x,
        bottom - e[1],
    );
    pb.line_to(x, y + a[1]);
    pb.cubic_to(x, y + a[1] - a[1] * k, x + a[0] - a[0] * k, y, x + a[0], y);
    pb.close();
}
pub(crate) fn path(b: Rect, r: Radii) -> Option<Path> {
    let mut pb = PathBuilder::new();
    append(&mut pb, b, r);
    pb.finish()
}
pub(crate) fn node_path(n: &SceneNode) -> Option<Path> {
    path(Rect::from_bounds(&n.bounds), corners(n))
}
/// Canvas roundRect overlap reduction for arbitrary elliptical shadow boxes.
pub(crate) fn reduced_radii(b: Rect, r: Radii) -> Radii {
    let mut f = 1.0_f32;
    for (side, sum) in [
        (b.w, r[0][0] + r[1][0]),
        (b.w, r[2][0] + r[3][0]),
        (b.h, r[0][1] + r[3][1]),
        (b.h, r[1][1] + r[2][1]),
    ] {
        if sum > 0.0 {
            f = f.min(side / sum);
        }
    }
    r.map(|r| [r[0] * f.max(0.0), r[1] * f.max(0.0)])
}
