use unicode_segmentation::UnicodeSegmentation;

/// Exact renderer pre-line wrapping: CR normalization; only TAB/FF/VT/ASCII SPACE
/// collapse. NBSP, narrow NBSP, WJ and BOM glue the entire word even when overfull.
pub fn wrap_text(text: &str, mut fits: impl FnMut(&str) -> bool) -> Vec<String> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = Vec::new();
    for paragraph in normalized.split('\n') {
        let mut line = String::new();
        for word in paragraph
            .split(['\t', '\u{000c}', '\u{000b}', ' '])
            .filter(|w| !w.is_empty())
        {
            let next = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if fits(&next) {
                line = next;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            if fits(word) || word.contains(['\u{00a0}', '\u{202f}', '\u{2060}', '\u{feff}']) {
                line = word.to_owned();
                continue;
            }
            for grapheme in word.graphemes(true) {
                if !line.is_empty() && !fits(&format!("{line}{grapheme}")) {
                    lines.push(std::mem::take(&mut line));
                }
                line.push_str(grapheme);
            }
        }
        lines.push(line);
    }
    lines
}
