//! Actual SVG gradients in frame-local user space; shape geometry is shared with clipping.
use super::{
    geometry::shape,
    markup::{attr, num},
    Node, Writer,
};
use serde_json::Value;
use std::fmt::Write;
impl Writer<'_> {
    pub(super) fn paint(&mut self, out: &mut String, node: &Node, opacity: f64) {
        if let Some(solid) = node.fill.as_str() {
            out.push_str(&shape(node.bounds, node.radius, Some((solid, opacity))));
            return;
        }
        let p = &node.fill;
        self.clips += 1;
        let id = format!("oc-gradient-{}", self.clips);
        let b = node.bounds;
        let linear = p["type"] == "linear";
        if linear {
            let angle = p["angle"].as_f64().unwrap_or(180.0).to_radians();
            let (dx, dy) = (angle.sin(), -angle.cos());
            let len = (b.width * dx).abs() + (b.height * dy).abs();
            let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
            let _=write!(out,"<defs><linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" color-interpolation=\"sRGB\">",num(x-dx*len/2.0),num(y-dy*len/2.0),num(x+dx*len/2.0),num(y+dy*len/2.0));
        } else {
            let cx = p["center"][0].as_f64().unwrap_or(0.5);
            let cy = p["center"][1].as_f64().unwrap_or(0.5);
            let (rx, ry) = if let Some(r) = p["radius"].as_f64() {
                (r, r)
            } else if p["radius"].is_array() {
                (number(&p["radius"][0]), number(&p["radius"][1]))
            } else {
                (
                    2.0_f64.sqrt() * (cx * b.width).abs().max((b.width - cx * b.width).abs()),
                    2.0_f64.sqrt() * (cy * b.height).abs().max((b.height - cy * b.height).abs()),
                )
            };
            let _=write!(out,"<defs><radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" cx=\"0\" cy=\"0\" r=\"{}\" gradientTransform=\"translate({} {}) scale({} 1)\" color-interpolation=\"sRGB\">",num(ry),num(b.x+cx*b.width),num(b.y+cy*b.height),num(rx/ry));
        }
        let stops: Vec<(f64, &str)> = p["stops"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|s| (number(&s[0]), s[1].as_str().unwrap_or("#0000")))
            .collect();
        out.push_str(&super::stops::markup(&stops));
        out.push_str(if linear {
            "</linearGradient></defs>"
        } else {
            "</radialGradient></defs>"
        });
        out.push_str(&shape(
            node.bounds,
            node.radius,
            Some((&format!("url(#{})", attr(&id)), opacity)),
        ));
    }
}
fn number(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}
