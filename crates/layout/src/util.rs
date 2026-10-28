use crate::RENDERER_VERSION;
use crate::model::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use taffy::prelude::*;

pub(crate) fn err(message: impl Into<String>) -> LayoutError {
    LayoutError {
        code: "LAYOUT_INVALID".into(),
        message: message.into(),
    }
}
pub(crate) fn hash<T: Serialize>(value: &T) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
}
pub(crate) fn quantize(n: f32) -> f32 {
    (n * 64.0).round() / 64.0
}
pub(crate) fn finite(n: f32) -> bool {
    n.is_finite() && n >= 0.0 && n <= 100_000.0
}
pub(crate) fn is_fill(n: &Option<Sizing>) -> bool {
    matches!(n, Some(Sizing::Rule(r)) if r == "fill")
}
pub(crate) fn dim(n: &Option<Sizing>, fill_percent: bool) -> Dimension {
    match n {
        Some(Sizing::Fixed(v)) => length(*v),
        Some(Sizing::Rule(r)) if r == "fill" && fill_percent => percent(1.0),
        _ => auto(),
    }
}

pub(crate) fn request(
    node: &AuthoredNode,
    fonts: &[Font],
    width: Option<f32>,
) -> Result<MeasureRequest, LayoutError> {
    let font_id = node.props.font_id.as_deref().unwrap_or("font_bundled");
    let font = fonts
        .iter()
        .find(|f| f.id == font_id)
        .ok_or_else(|| LayoutError {
            code: "FONT_MISSING".into(),
            message: format!("Font {font_id} has no manifest"),
        })?;
    let mut r = MeasureRequest {
        key: String::new(),
        text: node.props.text.clone().unwrap_or_default(),
        font_id: font.id.clone(),
        font_hash: font.hash.clone(),
        family: font.family.clone(),
        weight: font.weight,
        style: font.style.clone(),
        font_size: node.props.font_size.unwrap_or(16.0),
        line_height: node.props.line_height.unwrap_or(24.0),
        width: width.map(|w| quantize(w.max(0.0))),
    };
    r.key = hash(&(RENDERER_VERSION, &r));
    Ok(r)
}
