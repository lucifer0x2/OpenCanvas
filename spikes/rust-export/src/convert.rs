//! The layout crate's `Scene` as the spike painter's scene subset (scene.rs), plus a compact
//! geometry dump (bounds and text lines per node) for the Chrome/Rust gate in js/e2e/gate.mjs.
use crate::scene::{Bounds, Crop, Node, Provenance, Scene, Stroke, TextLine, Viewport};
use opencanvas_layout::Scene as LayoutScene;
use serde_json::{json, Value};

pub fn to_paint(s: &LayoutScene) -> Scene {
    let nodes = s
        .nodes
        .iter()
        .map(|n| Node {
            id: n.id.clone(),
            parent_id: n.parent_id.clone(),
            kind: n.kind.clone(),
            bounds: Bounds { x: n.bounds.x, y: n.bounds.y, width: n.bounds.width, height: n.bounds.height },
            lines: n.lines.iter().map(|l| TextLine { text: l.text.clone(), x: l.x, y: l.y }).collect(),
            font_size: n.font_size,
            fill: n.fill.clone(),
            fit: n.fit.clone(),
            opacity: n.opacity,
            clip: n.clip,
            radius: n.radius,
            crop: n.crop.as_ref().map(|c| Crop { x: c.x, y: c.y, width: c.width, height: c.height }),
            stroke: n.stroke.as_ref().map(|k| Stroke { color: k.color.clone(), width: k.width, align: k.align.clone() }),
        })
        .collect();
    let v = &s.provenance.viewport;
    Scene { provenance: Provenance { viewport: Viewport { width: v.width, height: v.height } }, nodes }
}

/// `[[id, x, y, width, height, [[text, x, y, width], ...]], ...]`, the fields the gate compares.
pub fn geometry(s: &LayoutScene) -> Value {
    let nodes: Vec<Value> = s
        .nodes
        .iter()
        .map(|n| {
            let b = &n.bounds;
            let lines: Vec<Value> = n.lines.iter().map(|l| json!([l.text, l.x, l.y, l.width])).collect();
            json!([n.id, b.x, b.y, b.width, b.height, lines])
        })
        .collect();
    json!(nodes)
}
