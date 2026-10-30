//! CSS shorthand, preserving the authored scalar or 1–4 array in resolved props.
use super::{values::number, Candidate};
use crate::error::*;
use serde_json::Value;

pub(super) fn quad(v: &Value, valid: impl Fn(&Value) -> bool) -> bool {
    match v {
        Value::Array(parts) => (1..=4).contains(&parts.len()) && parts.iter().all(valid),
        value => valid(value),
    }
}
pub(super) fn radius(v: &Value) -> Result<()> {
    if quad(v, |v| number(v, 0., 100_000., "radius").is_ok()) {
        Ok(())
    } else {
        Err(StorageError::validation("radius", "Radius must be a number 0..100000 or 1–4 corner values [top-left, top-right, bottom-right, bottom-left] with CSS shorthand"))
    }
}
impl Candidate {
    pub(super) fn resolve_quad(
        &self,
        value: &mut Value,
        expected: &str,
        field: &str,
    ) -> Result<()> {
        if value
            .as_object()
            .is_some_and(|o| !o.contains_key("tokenId") && !o.contains_key("styleId"))
        {
            let order = if field == "radius" {
                "top-left, top-right, bottom-right, bottom-left"
            } else {
                "top, right, bottom, left"
            };
            return Err(StorageError::validation(field, &format!("Use a number/color or a CSS shorthand array of 1–4 values [{order}], not an object with side keys")));
        }
        if let Value::Array(parts) = value {
            for v in parts.iter_mut().filter(|v| v.is_object()) {
                *v = self.resolve_value(v, expected).map_err(|mut e| {
                    e.field = Some(field.into());
                    e
                })?;
            }
        } else if value.is_object() {
            *value = self.resolve_value(value, expected).map_err(|mut e| {
                e.field = Some(field.into());
                e
            })?;
        }
        Ok(())
    }
}
