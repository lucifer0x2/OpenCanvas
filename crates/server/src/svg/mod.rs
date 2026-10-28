//! SVG markup of a computed scene, without a browser. It follows the renderer's paint rules
//! (`renderer/paint.ts`): scene order, fill then text or image, clip by every clipping ancestor,
//! and each node painted at the product of its own and its ancestors' opacity. Text keeps the
//! scene's measured lines and is not outlined; images embed their original bytes.
mod image_meta;
mod markup;
#[cfg(test)]
mod tests;
use crate::error::{ApiError, Result};
pub use image_meta::Image;
use markup::{attr, color, num, rect};
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
    writer.node(&mut out, root, 1.0, 0)?;
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
    fn node(&mut self, out: &mut String, node: &Node, parent: f64, depth: usize) -> Result<()> {
        if depth > self.depth {
            return Err(ApiError::new("SCENE_INVALID", "Scene cycle"));
        }
        let opacity = parent * node.opacity;
        if self.id_attribute {
            let _ = write!(out, "<g id=\"{}\">", attr(&node.id));
        }
        if node.kind != "text" && node.fill != "transparent" {
            out.push_str(&rect(node.bounds, node.radius, Some((&node.fill, opacity))));
        }
        match node.kind.as_str() {
            "text" => self.text(out, node, opacity),
            "image" => self.image(out, node, opacity)?,
            _ => {}
        }
        let children = self.children.get(node.id.as_str()).cloned();
        if let Some(children) = children.filter(|c| !c.is_empty()) {
            let clip = node
                .clip
                .then(|| self.clip_path(out, node.bounds, node.radius));
            if let Some(id) = &clip {
                let _ = write!(out, "<g clip-path=\"url(#{id})\">");
            }
            for child in children {
                self.node(out, child, opacity, depth + 1)?;
            }
            if clip.is_some() {
                out.push_str("</g>");
            }
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

    fn text(&mut self, out: &mut String, node: &Node, opacity: f64) {
        let font = node.font_id.as_deref().and_then(|id| self.fonts.get(id));
        let mut style = String::new();
        if let Some(font) = font {
            let family: String = font
                .family
                .chars()
                .filter(|c| !"'\"\\<>&".contains(*c))
                .collect();
            let _ = write!(style, " font-family=\"'{family}', sans-serif\"");
            let _ = write!(style, " font-weight=\"{}\"", attr(&font.weight.to_string()));
            if font.style != "normal" {
                let _ = write!(style, " font-style=\"{}\"", attr(&font.style));
            }
        } else {
            style.push_str(" font-family=\"sans-serif\"");
        }
        let _ = write!(
            style,
            " font-size=\"{}\"{}",
            num(node.font_size),
            color(&node.fill, opacity)
        );
        for line in &node.lines {
            self.has_text = true;
            let (x, y) = (num(node.bounds.x + line.x), num(node.bounds.y + line.y));
            let text = attr(&line.text);
            let _ = write!(
                out,
                "<text x=\"{x}\" y=\"{y}\"{style} xml:space=\"preserve\">{text}</text>"
            );
        }
    }

    fn image(&mut self, out: &mut String, node: &Node, opacity: f64) -> Result<()> {
        let image = node
            .asset_id
            .as_ref()
            .and_then(|id| self.images.get(id))
            .ok_or_else(|| ApiError::new("RESOURCE_NOT_READY", "Image asset is not loaded"))?;
        // The image viewport already clips to the bounds; a radius needs a rounded clip.
        let clip = (node.radius > 0.0).then(|| self.clip_path(out, node.bounds, node.radius));
        out.push_str("<g");
        if let Some(clip) = clip {
            let _ = write!(out, " clip-path=\"url(#{clip})\"");
        }
        if opacity < 1.0 {
            let _ = write!(out, " opacity=\"{}\"", num(opacity));
        }
        out.push('>');
        let fit = match node.fit.as_str() {
            "cover" => "xMidYMid slice",
            "fit" | "fill" => "none",
            _ => "xMidYMid meet",
        };
        let b = node.bounds;
        let place = format!(
            "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"{fit}\"",
            num(b.x),
            num(b.y),
            num(b.width),
            num(b.height)
        );
        let href = image.data_uri();
        match node.crop {
            None => {
                let _ = write!(out, "<image {place} href=\"{href}\"/>");
            }
            Some(c) => {
                let (w, h) = (f64::from(image.width), f64::from(image.height));
                let _ = write!(
                    out,
                    "<svg {place} viewBox=\"{} {} {} {}\"><image width=\"{}\" height=\"{}\" href=\"{href}\"/></svg>",
                    num(c.x * w),
                    num(c.y * h),
                    num(c.width * w),
                    num(c.height * h),
                    num(w),
                    num(h)
                );
            }
        }
        out.push_str("</g>");
        Ok(())
    }
}
