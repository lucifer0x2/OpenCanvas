//! Gradient stops whose straight-alpha SVG interpolation matches CSS premultiplied
//! interpolation (CSS Images §3.5.2), as `renderer/fill.ts` does for canvas. SVG renderers
//! interpolate color and opacity separately, so a stop fading to `transparent` (`#0000`)
//! would pass through gray: a zero-alpha stop takes each neighbor's color, and a segment
//! between unequal nonzero alphas is subdivided along the premultiplied curve.
use super::markup::num;
use std::fmt::Write;

type Rgba = [f64; 4];

fn rgba(hex: &str) -> Option<Rgba> {
    let h = hex.strip_prefix('#')?;
    let full: String = match h.len() {
        3 | 4 => h.chars().flat_map(|c| [c, c]).collect(),
        6 | 8 => h.into(),
        _ => return None,
    };
    let byte = |i: usize| {
        u8::from_str_radix(full.get(i..i + 2)?, 16)
            .ok()
            .map(f64::from)
    };
    let alpha = if full.len() == 8 {
        byte(6)? / 255.0
    } else {
        1.0
    };
    Some([byte(0)?, byte(2)?, byte(4)?, alpha])
}

/// Expands `[position, hex]` stops into premultiplied-equivalent `(position, rgba)` stops.
pub(super) fn expand(stops: &[(f64, &str)]) -> Vec<(f64, Rgba)> {
    let colors: Vec<Rgba> = stops
        .iter()
        .map(|(_, c)| rgba(c).unwrap_or([0.0; 4]))
        .collect();
    let mut out = Vec::new();
    for (i, (&(p, _), &a)) in stops.iter().zip(&colors).enumerate() {
        if a[3] == 0.0 {
            let prev = if i > 0 { colors[i - 1] } else { a };
            let next = colors.get(i + 1).copied().unwrap_or(a);
            out.push((p, [prev[0], prev[1], prev[2], 0.0]));
            out.push((p, [next[0], next[1], next[2], 0.0]));
        } else {
            out.push((p, a));
        }
        let Some(&(q, _)) = stops.get(i + 1) else {
            continue;
        };
        let b = colors[i + 1];
        if a[3] == 0.0 || b[3] == 0.0 || a[3] == b[3] || p == q {
            continue;
        }
        for j in 1..64 {
            let t = f64::from(j) / 64.0;
            let alpha = a[3] * (1.0 - t) + b[3] * t;
            let ch = |k: usize| (a[k] * a[3] * (1.0 - t) + b[k] * b[3] * t) / alpha;
            out.push((p + (q - p) * t, [ch(0), ch(1), ch(2), alpha]));
        }
    }
    out
}

/// `<stop>` elements for SVG 1.1 consumers: six-digit hex plus `stop-opacity`.
pub(super) fn markup(stops: &[(f64, &str)]) -> String {
    let mut out = String::new();
    for (at, [r, g, b, a]) in expand(stops) {
        let hex = |v: f64| v.round().clamp(0.0, 255.0) as u8;
        let _ = write!(
            out,
            "<stop offset=\"{}\" stop-color=\"#{:02x}{:02x}{:02x}\"",
            num(at),
            hex(r),
            hex(g),
            hex(b)
        );
        if a < 1.0 {
            let _ = write!(out, " stop-opacity=\"{}\"", num(a));
        }
        out.push_str("/>");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transparent_stops_take_neighbor_colors_and_unequal_alpha_is_subdivided() {
        let red_clear = markup(&[(0.0, "#f00"), (1.0, "#0000")]);
        assert_eq!(red_clear, "<stop offset=\"0\" stop-color=\"#ff0000\"/><stop offset=\"1\" stop-color=\"#ff0000\" stop-opacity=\"0\"/><stop offset=\"1\" stop-color=\"#000000\" stop-opacity=\"0\"/>");
        let mixed = expand(&[(0.0, "#ff000080"), (1.0, "#0000ffcc")]);
        assert_eq!(mixed.len(), 65);
        let mid = mixed[32].1;
        // Premultiplied midpoint: red 0.502·255/2 over alpha 0.651.
        assert!(
            (mid[0] - 98.3).abs() < 0.5 && (mid[2] - 156.7).abs() < 0.5,
            "{mid:?}"
        );
        assert_eq!(
            expand(&[(0.0, "#f00"), (0.5, "#00f"), (0.5, "#0f0"), (1.0, "#fff")]).len(),
            4
        );
    }
}
