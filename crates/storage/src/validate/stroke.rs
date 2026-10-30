//! The `stroke` property: `{color, width?, align?}` where color and width take one value
//! or exactly 4 in CSS order [top, right, bottom, left].
use super::{values::*, Candidate};
use crate::error::*;
use serde_json::Value;

const SIDES: &str = "1–4 of them in CSS shorthand [top, right, bottom, left]";

impl Candidate {
    /// Replaces token and style references in `stroke.color` and `stroke.width` with
    /// literals. A malformed shape is left as authored for `validate_stroke` to report.
    pub(super) fn resolve_stroke(&self, stroke: &mut Value) -> Result<()> {
        let Some(stroke) = stroke.as_object_mut() else {
            return Ok(());
        };
        for (key, expected) in [("color", "color"), ("width", "dimension")] {
            let Some(value) = stroke.get_mut(key) else {
                continue;
            };
            self.resolve_quad(value, expected, &format!("stroke.{key}"))?;
        }
        Ok(())
    }
}

/// Checks a resolved `stroke` value. Errors name `stroke.<key>`, never `props.stroke`,
/// which is reserved for the key-level check.
pub(super) fn validate_stroke(v: &Value) -> Result<()> {
    let stroke = v.as_object().ok_or_else(|| {
        StorageError::validation("stroke", "Stroke must be an object {color, width?, align?}")
    })?;
    if let Some(key) = stroke
        .keys()
        .find(|k| !matches!(k.as_str(), "color" | "width" | "align"))
    {
        return Err(StorageError::validation(
            &format!("stroke.{key}"),
            &format!("Unsupported stroke key '{key}'; stroke takes color, width and align"),
        ));
    }
    let color_message = format!(
        "Stroke color must be #RGB, #RGBA, #RRGGBB or #RRGGBBAA, a color token, a fill style, or {SIDES}"
    );
    let color_value = stroke
        .get("color")
        .ok_or_else(|| StorageError::validation("stroke.color", "Stroke requires color"))?;
    super::quad::quad(color_value, |c| color(c).is_ok())
        .then_some(())
        .ok_or_else(|| StorageError::validation("stroke.color", &color_message))?;
    if let Some(width) = stroke.get("width") {
        let message =
            format!("Stroke width must be a number 0..100000, a dimension token, or {SIDES}");
        super::quad::quad(width, |w| number(w, 0., 100_000., "stroke.width").is_ok())
            .then_some(())
            .ok_or_else(|| StorageError::validation("stroke.width", &message))?;
    }
    if let Some(align) = stroke.get("align") {
        if !matches!(align.as_str(), Some("inside" | "center" | "outside")) {
            return Err(StorageError::validation(
                "stroke.align",
                "Stroke align must be inside, center or outside",
            ));
        }
    }
    Ok(())
}
