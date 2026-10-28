//! SVG markup of a computed scene, without a browser. It follows the renderer's paint rules
//! (`renderer/paint.ts`): scene order, fill then text or image, clip by every clipping ancestor,
//! and CSS group opacity (`renderer/layers.ts`). Text keeps the scene's measured lines and is not
//! outlined; images embed their original bytes.
mod elements;
mod image_meta;
mod markup;
#[cfg(test)]
mod tests;
use crate::error::{ApiError, Result};
pub use image_meta::Image;
use markup::{attr, num, rect};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, fmt::Write};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Node {
    id: String,
    parent_id: Option<String>,
    kind: String,
    bounds: Bounds,
    #[serde(default)]
    lines: Vec<Line>,
    font_id: Option<String>,
    #[serde(default)]
    font_size: f64,
    #[serde(default)]
    fill: String,
    asset_id: Option<String>,
    #[serde(default)]
    fit: String,
    #[serde(default = "one")]
    opacity: f64,
    #[serde(default)]
    clip: bool,
    #[serde(default)]
    radius: f64,
    crop: Option<Bounds>,
}

#[derive(Deserialize, Clone, Copy)]
struct Bounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Deserialize)]
struct Line {
    text: String,
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
struct Font {
    id: String,
    family: String,
    weight: Value,
    style: String,
}

fn one() -> f64 {
    1.0
}

/// The markup and whether it holds text, which renders in whatever font the viewer has.
pub struct Svg {
    pub markup: String,
    pub has_text: bool,
}

/// `images` holds every image asset the scene paints, by asset ID. With `id_attribute`, each
/// node's elements sit in a `<g id="<nodeId>">`.
pub fn scene_to_svg(
    scene: &Value,
    images: &HashMap<String, Image>,
    id_attribute: bool,
) -> Result<Svg> {
    let invalid = |_| ApiError::new("SCENE_INVALID", "Scene cannot be converted to SVG");
    let nodes: Vec<Node> = serde_json::from_value(scene["nodes"].clone()).map_err(invalid)?;
    let fonts: Vec<Font> = serde_json::from_value(scene["fonts"].clone()).unwrap_or_default();
    let root = nodes
        .iter()
        .find(|n| n.parent_id.is_none())
        .ok_or_else(|| ApiError::new("SCENE_INVALID", "Scene has no root node"))?;
    let mut children: HashMap<&str, Vec<&Node>> = HashMap::new();
    for node in &nodes {
        if let Some(parent) = &node.parent_id {
            children.entry(parent).or_default().push(node);
        }
    }
    let (w, h) = (num(root.bounds.width), num(root.bounds.height));
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">"
    );
    let mut writer = Writer {
        fonts: fonts.iter().map(|f| (f.id.as_str(), f)).collect(),
        children,
        images,
        id_attribute,
        clips: 0,
        has_text: false,
        depth: nodes.len(),
    };
    writer.node(&mut out, root, 0)?;
    out.push_str("</svg>");
    Ok(Svg {
        markup: out,
        has_text: writer.has_text,
    })
}

struct Writer<'a> {
    fonts: HashMap<&'a str, &'a Font>,
    children: HashMap<&'a str, Vec<&'a Node>>,
    images: &'a HashMap<String, Image>,
    id_attribute: bool,
    clips: usize,
    has_text: bool,
    depth: usize,
}

impl Writer<'_> {
    fn node(&mut self, out: &mut String, node: &Node, depth: usize) -> Result<()> {
        if depth > self.depth {
            return Err(ApiError::new("SCENE_INVALID", "Scene cycle"));
        }
        let children = self.children.get(node.id.as_str()).cloned();
        // CSS group opacity, as the renderer composites it: a node with children wraps its
        // subtree in one `<g opacity>`; a leaf carries its opacity on its own elements.
        let group = node.opacity < 1.0 && children.as_ref().is_some_and(|c| !c.is_empty());
        let opacity = if group { 1.0 } else { node.opacity };
        if self.id_attribute {
            let _ = write!(out, "<g id=\"{}\">", attr(&node.id));
        }
        if group {
            let _ = write!(out, "<g opacity=\"{}\">", num(node.opacity));
        }
        if node.kind != "text" && node.fill != "transparent" {
            out.push_str(&rect(node.bounds, node.radius, Some((&node.fill, opacity))));
        }
        match node.kind.as_str() {
            "text" => self.text(out, node, opacity),
            "image" => self.image(out, node, opacity)?,
            _ => {}
        }
        if let Some(children) = children.filter(|c| !c.is_empty()) {
            let clip = node
                .clip
                .then(|| self.clip_path(out, node.bounds, node.radius));
            if let Some(id) = &clip {
                let _ = write!(out, "<g clip-path=\"url(#{id})\">");
            }
            for child in children {
                self.node(out, child, depth + 1)?;
            }
            if clip.is_some() {
                out.push_str("</g>");
            }
        }
        if group {
            out.push_str("</g>");
        }
        if self.id_attribute {
            out.push_str("</g>");
        }
        Ok(())
    }

    /// Writes a rounded-rect `<clipPath>` and returns its ID, prefixed apart from node IDs.
    fn clip_path(&mut self, out: &mut String, bounds: Bounds, radius: f64) -> String {
        self.clips += 1;
        let id = format!("oc-clip-{}", self.clips);
        let _ = write!(
            out,
            "<clipPath id=\"{id}\">{}</clipPath>",
            rect(bounds, radius, None)
        );
        id
    }
}
