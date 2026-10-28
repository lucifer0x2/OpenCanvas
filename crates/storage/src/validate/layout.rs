use super::{values::number, Candidate};
use crate::{error::*, model::*};
use serde_json::{Map, Value};

impl Candidate {
    pub(super) fn layout_properties(&self, n: &Node, p: &Map<String, Value>) -> Result<()> {
        for key in ["width", "height"] {
            if let Some(v) = p.get(key) {
                if !matches!(v.as_str(), Some("fill" | "hug")) {
                    number(v, 0., 100_000., key)?;
                }
                if v == "fill" {
                    if let Some(parent) = n.parent_id.as_ref().and_then(|id| self.nodes.get(id)) {
                        let props = self.resolve_props(parent)?;
                        if props.get(key).is_none() || props.get(key).is_some_and(|v| v == "hug") {
                            return Err(StorageError::validation(
                                key,
                                "Fill inside a hug axis is ambiguous and unsupported",
                            ));
                        }
                    }
                }
            }
        }
        for key in ["minWidth", "maxWidth", "minHeight", "maxHeight"] {
            if let Some(v) = p.get(key) {
                number(v, 0., 100_000., key)?;
            }
        }
        for axis in ["Width", "Height"] {
            if let (Some(min), Some(max)) = (
                p.get(&format!("min{axis}")).and_then(Value::as_f64),
                p.get(&format!("max{axis}")).and_then(Value::as_f64),
            ) {
                if min > max {
                    return Err(StorageError::validation(
                        "bounds",
                        "Minimum must not exceed maximum",
                    ));
                }
            }
        }
        if let Some(v) = p.get("layout") {
            if !n.kind.container() {
                return Err(StorageError::validation(
                    "layout",
                    "Only containers support layout",
                ));
            }
            let l = v
                .as_object()
                .ok_or_else(|| StorageError::validation("layout", "Layout must be an object"))?;
            if !matches!(
                l.get("mode").and_then(Value::as_str),
                Some("row" | "column" | "free")
            ) {
                return Err(StorageError::validation(
                    "layout.mode",
                    "Only row, column and free layout are supported",
                ));
            }
            for (key, value) in l {
                match key.as_str() {
                    "mode" => {}
                    "padding" | "gap" => number(value, 0., 100_000., key)?,
                    "wrap" if value.is_boolean() => {}
                    "wrap" => {
                        return Err(StorageError::validation(
                            "layout.wrap",
                            "Wrap must be a literal boolean",
                        ))
                    }
                    _ => {
                        return Err(StorageError::validation(
                            "layout",
                            "Unsupported layout property",
                        ))
                    }
                }
            }
            if l.get("wrap") == Some(&Value::Bool(true)) {
                if l.get("mode").and_then(Value::as_str) != Some("row") {
                    return Err(StorageError::validation(
                        "layout.wrap",
                        "Wrap requires row mode",
                    ));
                }
                self.definite_wrap_width(n)?;
            }
        }
        if let Some(parent) = n.parent_id.as_ref().and_then(|id| self.nodes.get(id)) {
            let parent = self.resolve_props(parent)?;
            if parent.pointer("/layout/wrap") == Some(&Value::Bool(true))
                && p.get("width").is_some_and(|v| v == "fill")
                && p.get("minWidth")
                    .and_then(Value::as_f64)
                    .is_none_or(|v| v <= 0.)
            {
                return Err(StorageError::validation(
                    "minWidth",
                    "Wrapping row fill child requires explicit positive minWidth",
                ));
            }
        }
        Ok(())
    }

    fn definite_wrap_width(&self, n: &Node) -> Result<()> {
        let mut current = n;
        // Node validation checks cycles before calling property validation.
        loop {
            let props = self.resolve_props(current)?;
            match props.get("width") {
                Some(v) if v.as_f64().is_some_and(|v| v > 0.) => return Ok(()),
                Some(v) if v == "fill" => {
                    current = current
                        .parent_id
                        .as_ref()
                        .and_then(|id| self.nodes.get(id))
                        .ok_or_else(|| {
                            StorageError::validation(
                                "width",
                                "Wrap requires definite positive width",
                            )
                        })?;
                }
                _ => {
                    return Err(StorageError::validation(
                        "width",
                        "Wrap requires definite positive width",
                    ))
                }
            }
        }
    }
}
