//! CSS box-shadow layers, in authored top-to-bottom order.
use super::{values::*, Candidate};
use crate::error::*;
use serde_json::Value;

impl Candidate {
    pub(super) fn resolve_shadow(&self, value: &mut Value) -> Result<()> {
        for (i, layer) in value.as_array_mut().into_iter().flatten().enumerate() {
            for key in ["x", "y", "blur", "spread", "color"] {
                if let Some(v) = layer.get_mut(key) {
                    let expected = match key {
                        "color" => "color",
                        "blur" => "dimension",
                        _ => "number",
                    };
                    *v = self.resolve_value(v, expected).map_err(|mut e| {
                        e.field = Some(format!("shadow[{i}].{key}"));
                        e
                    })?;
                }
            }
        }
        Ok(())
    }
}
pub(super) fn shadow(v: &Value) -> Result<()> {
    let layers = v.as_array().filter(|a| a.len() <= 8).ok_or_else(|| StorageError::validation("shadow", "Shadow is an array of up to 8 layers [{x, y, blur, spread, color, inset}], first on top like CSS box-shadow"))?;
    for (i, layer) in layers.iter().enumerate() {
        let field = |k: &str| format!("shadow[{i}].{k}");
        let layer = layer
            .as_object()
            .ok_or_else(|| StorageError::validation("shadow", "Shadow layers must be objects"))?;
        if let Some(key) = layer
            .keys()
            .find(|k| !["x", "y", "blur", "spread", "color", "inset"].contains(&k.as_str()))
        {
            return Err(StorageError::validation(
                &field(key),
                "Unsupported shadow key; a layer takes x, y, blur, spread, color and inset",
            ));
        }
        let ink = layer.get("color").ok_or_else(|| {
            StorageError::validation(&field("color"), "Shadow layer requires color")
        })?;
        color(ink).map_err(|_| {
            StorageError::validation(&field("color"), "Shadow color must be a solid hex color")
        })?;
        for (key, min, max) in [
            ("x", -10000., 10000.),
            ("y", -10000., 10000.),
            ("blur", 0., 1000.),
            ("spread", -1000., 1000.),
        ] {
            if let Some(v) = layer.get(key) {
                number(v, min, max, &field(key))?;
            }
        }
        if layer.get("inset").is_some_and(|v| !v.is_boolean()) {
            return Err(StorageError::validation(
                &field("inset"),
                "Shadow inset must be true or false",
            ));
        }
    }
    Ok(())
}
