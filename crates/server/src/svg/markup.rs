//! Small SVG markup pieces: numbers, escaped text, colors and rounded rects.
use super::Bounds;

/// A coordinate or size with at most three decimals and no trailing zeros.
pub fn num(value: f64) -> String {
    let fixed = format!("{value:.3}");
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    match trimmed {
        "-0" | "" => "0".into(),
        other => other.into(),
    }
}

/// Escapes text content and attribute values alike. Characters XML 1.0 forbids even as
/// references (C0 controls other than tab/newline/CR, U+FFFE, U+FFFF) are dropped; Rust
/// strings hold no surrogates.
pub fn attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\t' | '\n' | '\r' => out.push(c),
            '\u{0}'..='\u{1f}' | '\u{fffe}' | '\u{ffff}' => {}
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// ` fill="#RRGGBB"` plus `fill-opacity` for a hex alpha and `opacity` below 1. SVG 1.1 and
/// many editors do not read 8-digit hex, so `#RGBA`/`#RRGGBBAA` are split.
pub fn color(fill: &str, opacity: f64) -> String {
    let mut out = match split_hex(fill) {
        Some((rgb, 255)) => format!(" fill=\"#{rgb}\""),
        Some((rgb, alpha)) => format!(
            " fill=\"#{rgb}\" fill-opacity=\"{}\"",
            num(f64::from(alpha) / 255.0)
        ),
        None => format!(" fill=\"{}\"", attr(fill)),
    };
    if opacity < 1.0 {
        out.push_str(&format!(" opacity=\"{}\"", num(opacity)));
    }
    out
}

/// `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA` as six lowercase hex digits and an alpha byte.
fn split_hex(fill: &str) -> Option<(String, u8)> {
    let hex = fill.strip_prefix('#')?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let full: String = match hex.len() {
        3 | 4 => hex.chars().flat_map(|c| [c, c]).collect(),
        6 | 8 => hex.into(),
        _ => return None,
    };
    let full = full.to_ascii_lowercase();
    let alpha = match full.get(6..8) {
        Some(a) => u8::from_str_radix(a, 16).ok()?,
        None => 255,
    };
    Some((full[..6].into(), alpha))
}

/// A `<rect>` with the renderer's corner rule: a radius larger than half a side shrinks to it,
/// as Canvas `roundRect` does. `paint` is the fill and opacity; `None` for a clip shape.
pub fn rect(b: Bounds, radius: f64, paint: Option<(&str, f64)>) -> String {
    let r = radius.min(b.width / 2.0).min(b.height / 2.0).max(0.0);
    let mut out = format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
        num(b.x),
        num(b.y),
        num(b.width),
        num(b.height)
    );
    if r > 0.0 {
        out.push_str(&format!(" rx=\"{}\"", num(r)));
    }
    if let Some((fill, opacity)) = paint {
        out.push_str(&color(fill, opacity));
    }
    out.push_str("/>");
    out
}
