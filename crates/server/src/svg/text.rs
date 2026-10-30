//! SVG text adapter: use cached line origins, tracking and decoration geometry.
use super::markup::{attr, color, num};
use super::{Node, Writer};
use std::fmt::Write;

// Scene text uses a 1/64px grid: six decimals preserve every cached origin.
fn coordinate(value: f64) -> String {
    let fixed = format!("{value:.6}");
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" {
        "0".into()
    } else {
        trimmed.into()
    }
}

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
            let family = attr(&family);
            let _ = write!(
                style,
                " font-family=\"'{family}'\" font-weight=\"{}\"",
                attr(&font.weight.to_string())
            );
            if font.style != "normal" {
                let _ = write!(style, " font-style=\"{}\"", attr(&font.style));
            }
        } else {
            style.push_str(" font-family=\"sans-serif\"");
        }
        let _ = write!(
            style,
            " font-size=\"{}\" letter-spacing=\"{}\" style=\"white-space:pre;font-kerning:normal\"{}",
            num(node.font_size),
            num(node.letter_spacing),
            color(node.fill.as_str().unwrap_or("transparent"), opacity)
        );
        for line in &node.lines {
            self.has_text = true;
            let (x, y) = (
                coordinate(node.bounds.x + line.x),
                coordinate(node.bounds.y + line.y),
            );
            let text = attr(&line.text);
            let _ = write!(
                out,
                "<text x=\"{x}\" y=\"{y}\"{style} xml:space=\"preserve\">{text}</text>"
            );
            if let Some(d) = &node.decoration {
                let _ = write!(
                    out,
                    "<rect x=\"{x}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>",
                    coordinate((node.bounds.y + line.y + d.offset).round()),
                    coordinate(line.width),
                    coordinate(d.thickness),
                    color(node.fill.as_str().unwrap_or("transparent"), opacity)
                );
            }
        }
    }
}
