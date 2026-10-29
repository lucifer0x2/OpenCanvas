//! Subset of renderer/types.ts Scene that the spike scene uses, plus the paint-unit index.
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Copy)]
pub struct Bounds { pub x: f32, pub y: f32, pub width: f32, pub height: f32 }

#[derive(Deserialize, Clone)]
pub struct TextLine { pub text: String, pub x: f32, pub y: f32 }

#[derive(Deserialize, Clone)]
pub struct Stroke { pub color: [String; 4], pub width: [f32; 4], pub align: String }

#[derive(Deserialize, Clone)]
pub struct Crop { pub x: f32, pub y: f32, pub width: f32, pub height: f32 }

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: String,
    pub bounds: Bounds,
    pub lines: Vec<TextLine>,
    pub font_size: f32,
    pub fill: String,
    pub fit: String,
    pub opacity: f32,
    pub clip: bool,
    pub radius: f32,
    pub crop: Option<Crop>,
    pub stroke: Option<Stroke>,
}

#[derive(Deserialize)]
pub struct Viewport { pub width: f32, pub height: f32 }
#[derive(Deserialize)]
pub struct Provenance { pub viewport: Viewport }
#[derive(Deserialize)]
pub struct Scene { pub provenance: Provenance, pub nodes: Vec<Node> }

/// Tree order plus paint units (own paint, stroke, each painting child) like scene-tree.ts.
pub struct Tree { pub roots: Vec<usize>, pub children: HashMap<usize, Vec<usize>>, pub units: Vec<u32> }

pub fn paints_itself(n: &Node) -> bool {
    n.kind == "text" || n.kind == "image" || n.fill != "transparent"
}

pub fn has_stroke(n: &Node) -> bool {
    n.stroke.as_ref().is_some_and(|s| s.width.iter().any(|w| *w > 0.0))
}

pub fn index(scene: &Scene) -> Tree {
    let pos: HashMap<&str, usize> = scene.nodes.iter().enumerate().map(|(i, n)| (n.id.as_str(), i)).collect();
    let mut t = Tree { roots: vec![], children: HashMap::new(), units: vec![0; scene.nodes.len()] };
    for (i, n) in scene.nodes.iter().enumerate() {
        match n.parent_id.as_deref().and_then(|p| pos.get(p)) {
            Some(&p) => t.children.entry(p).or_default().push(i),
            None => t.roots.push(i),
        }
    }
    fn visit(i: usize, s: &Scene, t: &mut Tree) -> bool {
        let n = &s.nodes[i];
        let mut u = paints_itself(n) as u32 + has_stroke(n) as u32;
        for c in t.children.get(&i).cloned().unwrap_or_default() {
            if visit(c, s, t) { u += 1; }
        }
        t.units[i] = u;
        u > 0
    }
    for r in t.roots.clone() { visit(r, scene, &mut t); }
    t
}

/// "#rgb", "#rrggbb", "#rrggbbaa" → straight RGBA; "transparent" → None.
pub fn parse_color(c: &str) -> Option<[u8; 4]> {
    let h = c.strip_prefix('#')?;
    let v = |s: &str| u8::from_str_radix(s, 16).ok();
    match h.len() {
        3 => { let d: Vec<u8> = h.chars().map(|c| v(&c.to_string()).unwrap() * 17).collect(); Some([d[0], d[1], d[2], 255]) }
        6 => Some([v(&h[0..2])?, v(&h[2..4])?, v(&h[4..6])?, 255]),
        8 => Some([v(&h[0..2])?, v(&h[2..4])?, v(&h[4..6])?, v(&h[6..8])?]),
        _ => None,
    }
}
