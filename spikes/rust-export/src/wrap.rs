//! Port of renderer/measurement.ts measureBatch wrapping onto HarfRust widths.
use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

pub fn round64(v: f64) -> f64 {
    (v * 64.0).round() / 64.0
}

#[derive(Serialize, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    pub y: f64,
    pub width: f64,
}

/// JS `\s` minus LF, close enough for the corpus (no exotic separators in it).
fn is_ws(c: char) -> bool {
    c != '\n' && (c.is_whitespace() || c == '\u{feff}')
}

/// `px` returns the unrounded advance width of a string at `size`.
pub fn measure(px: &dyn Fn(&str) -> f64, text: &str, size: f64, lh: f64, width: Option<f64>) -> Vec<Line> {
    let max = width.map(|w| w.max(0.0)).unwrap_or(f64::INFINITY);
    let w = |t: &str| round64(px(t));
    let fits = |t: &str| w(t) <= max;
    let mut lines: Vec<Line> = Vec::new();
    let add = |lines: &mut Vec<Line>, t: &str| {
        let y = round64(lines.len() as f64 * lh + size + (lh - size) / 2.0);
        lines.push(Line { text: t.to_string(), y, width: w(t) });
    };
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    for paragraph in normalized.split('\n') {
        let words: Vec<&str> = paragraph
            .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .split(is_ws)
            .filter(|s| !s.is_empty())
            .collect();
        let mut line = String::new();
        for word in words {
            let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if fits(&next) {
                line = next;
                continue;
            }
            if !line.is_empty() {
                add(&mut lines, &line);
                line.clear();
            }
            if fits(word) {
                line = word.to_string();
                continue;
            }
            for g in word.graphemes(true) {
                if !line.is_empty() && !fits(&format!("{line}{g}")) {
                    add(&mut lines, &line);
                    line.clear();
                }
                line.push_str(g);
            }
        }
        add(&mut lines, &line);
    }
    lines
}
