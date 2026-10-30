//! Four-corner circular geometry with CSS's common overlap reduction factor.
use super::{
    markup::{color, num, rect},
    Bounds,
};
use serde::Deserialize;
#[derive(Clone, Copy, Deserialize)]
#[serde(untagged)]
pub(super) enum Radius {
    Scalar(f64),
    Corners([f64; 4]),
}
impl Default for Radius {
    fn default() -> Self {
        Self::Scalar(0.0)
    }
}
impl Radius {
    pub(super) fn each(self) -> [f64; 4] {
        match self {
            Self::Scalar(v) => [v; 4],
            Self::Corners(r) => r,
        }
    }
}
pub(super) fn shape(b: Bounds, radius: Radius, paint: Option<(&str, f64)>) -> String {
    let r = radius.each();
    if r.iter().all(|v| *v == r[0]) {
        return rect(b, r[0], paint);
    }
    let [a, c, d, e] = r;
    let ratio = |v: f64, s: f64| if s > 0.0 { v / s } else { 1.0 };
    let f = 1.0_f64
        .min(ratio(b.width, a + c))
        .min(ratio(b.width, d + e))
        .min(ratio(b.height, a + e))
        .min(ratio(b.height, c + d))
        .max(0.0);
    let [tl, tr, br, bl] = r.map(|v| v * f);
    let (x, y, w, h) = (b.x, b.y, b.width, b.height);
    let n = num;
    let path=format!("M {} {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} Z",
        n(x+tl),n(y),n(x+w-tr),n(tr),n(tr),n(x+w),n(y+tr),n(y+h-br),n(br),n(br),n(x+w-br),n(y+h),n(x+bl),n(bl),n(bl),n(x),n(y+h-bl),n(y+tl),n(tl),n(tl),n(x+tl),n(y));
    format!(
        "<path d=\"{path}\"{}/>",
        paint.map_or(String::new(), |(p, a)| color(p, a))
    )
}
