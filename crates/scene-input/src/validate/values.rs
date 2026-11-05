use crate::error::*;
use serde_json::Value;

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
pub fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty()
        || slug.len() > 64
        || !slug
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        || slug.starts_with('-')
        || slug.ends_with('-')
    {
        return Err(StorageError::validation(
            "slug",
            "Use 1–64 lowercase letters, digits or internal hyphens",
        ));
    }
    Ok(())
}
pub(super) fn id(id: &str) -> Result<()> {
    if valid_id(id) {
        Ok(())
    } else {
        Err(StorageError::validation(
            "id",
            "ID must contain 1–128 ASCII letters, digits, underscores or hyphens",
        ))
    }
}
pub(super) fn number(v: &Value, min: f64, max: f64, field: &str) -> Result<()> {
    if let Some(n) = v.as_f64() {
        if n.is_finite() && n >= min && n <= max {
            return Ok(());
        }
    }
    Err(StorageError::validation(
        field,
        "Expected a finite number within the supported range",
    ))
}
pub(super) fn color(v: &Value) -> Result<()> {
    let Some(s) = v.as_str() else {
        return Err(StorageError::validation(
            "color",
            "Color must be a literal hex color",
        ));
    };
    let hex = s.strip_prefix('#').unwrap_or("");
    if [3, 4, 6, 8].contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(StorageError::validation(
            "color",
            "Color must be #RGB, #RGBA, #RRGGBB or #RRGGBBAA",
        ))
    }
}
pub(super) fn literal(v: &Value, kind: &str) -> Result<()> {
    match kind {
        "color" => color(v),
        "dimension" => number(v, 0., 100_000., "dimension"),
        "number" => number(v, -100_000., 100_000., "number"),
        _ => Err(StorageError::validation(
            "type",
            "Only literal color, dimension and number tokens are supported",
        )),
    }
}
