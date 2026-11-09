//! Shared text-style and node checks; metrics enums deliberately have no token type.
use super::values::number;
use crate::error::*;
use serde_json::Value;

pub(super) fn text_property(key: &str, value: &Value) -> Result<()> {
    let allowed: &[&str] = match key {
        "letterSpacing" => return number(value, -1000., 1000., key),
        "textWrap" => &["wrap", "nowrap"],
        "textAlign" => &["left", "center", "right"],
        "textDecoration" => &["none", "underline", "line-through"],
        "textTransform" => &["none", "uppercase", "lowercase", "capitalize"],
        _ => unreachable!("text property dispatch"),
    };
    if value.as_str().is_some_and(|s| allowed.contains(&s)) {
        Ok(())
    } else {
        Err(StorageError::validation(
            key,
            &format!("{key} must be {}", allowed.join(" or ")),
        ))
    }
}
