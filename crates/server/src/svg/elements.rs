//! Text and image elements of one scene node.
use super::markup::{attr, color, num};
use super::{Node, Writer};
use crate::error::{ApiError, Result};
use std::fmt::Write;

impl Writer<'_> {
    pub(super) fn text(&mut self, out: &mut String, node: &Node, opacity: f64) {
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

    pub(super) fn image(&mut self, out: &mut String, node: &Node, opacity: f64) -> Result<()> {
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
