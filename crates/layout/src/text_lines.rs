//! Position measured lines in their final box; never remeasure or mutate source strings.
use crate::model::*;
use crate::util::err;

pub(crate) fn validate(p: &Props) -> Result<(), LayoutError> {
    for (value, allowed) in [
        (p.text_wrap.as_deref(), &["wrap", "nowrap"][..]),
        (p.text_align.as_deref(), &["left", "center", "right"][..]),
        (
            p.text_decoration.as_deref(),
            &["none", "underline", "line-through"][..],
        ),
        (
            p.text_transform.as_deref(),
            &["none", "uppercase", "lowercase", "capitalize"][..],
        ),
    ] {
        if value.is_some_and(|v| !allowed.contains(&v)) {
            return Err(err("Unsupported text property value"));
        }
    }
    if p.letter_spacing
        .is_some_and(|v| !v.is_finite() || !(-1000.0..=1000.0).contains(&v))
    {
        return Err(err("Letter spacing must be finite pixels -1000..1000"));
    }
    Ok(())
}

pub(crate) fn align(m: &Measurement, p: &Props, width: f32) -> Vec<TextLine> {
    m.lines
        .iter()
        .cloned()
        .map(|mut line| {
            let spare = (width - line.width).max(0.0);
            line.x = match p.text_align.as_deref() {
                Some("center") => (spare * 32.0).floor() / 64.0,
                Some("right") => spare,
                _ => 0.0,
            };
            line
        })
        .collect()
}

pub(crate) fn decoration(p: &Props, ascent: f32) -> Option<SceneDecoration> {
    let line = p.text_decoration.as_deref().unwrap_or("none");
    let t = (p.font_size.unwrap_or(16.0) / 10.0).max(1.0);
    let offset = match line {
        "underline" => (t / 2.0).ceil().max(1.0),
        "line-through" => -ascent / 3.0 - t / 2.0,
        _ => return None,
    };
    Some(SceneDecoration {
        line: line.into(),
        offset,
        thickness: t.trunc().max(1.0),
    })
}

pub(crate) fn diagnostics(out: &mut Vec<String>, node: &AuthoredNode, m: &Measurement) {
    let font = node.props.font_id.as_deref().unwrap_or("font_bundled");
    if m.coverage_unavailable {
        out.push(format!("GLYPH_CHECK_UNAVAILABLE node={} font={font}: cmap format unavailable; coverage not verified", node.id));
    }
    if !m.missing.is_empty() {
        let codes = m
            .missing
            .iter()
            .take(8)
            .map(|c| format!("U+{c:04X}"))
            .collect::<Vec<_>>()
            .join(", ");
        let more = if m.missing.len() > 8 {
            format!(" +{} more", m.missing.len() - 8)
        } else {
            String::new()
        };
        out.push(format!("MISSING_GLYPH node={} font={font}: cmap has no mapping for {codes}{more}; fallback and cluster shaping unverified", node.id));
    }
}

pub(crate) fn cap(mut diagnostics: Vec<String>) -> Vec<String> {
    let omitted = diagnostics.len().saturating_sub(20);
    diagnostics.truncate(20);
    if omitted > 0 {
        diagnostics.push(format!("GLYPH_DIAGNOSTICS_LIMIT +{omitted} more warnings"));
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_measurements_default_new_fields_and_diagnostics_are_bounded() {
        let old: Measurement =
            serde_json::from_str(r#"{"width":1,"height":2,"lines":[]}"#).unwrap();
        assert!(old.missing.is_empty());
        assert!(!old.coverage_unavailable);
        let warnings = cap((0..25).map(|i| format!("MISSING_GLYPH {i}")).collect());
        assert_eq!(warnings.len(), 21);
        assert_eq!(
            warnings.last().unwrap(),
            "GLYPH_DIAGNOSTICS_LIMIT +5 more warnings"
        );
    }
}
