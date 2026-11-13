use super::{breakpoints, layout_rule::validate_layout_rule, scope::Resolver, values::number};
use crate::{error::*, model::*};
use opencanvas_layout::percentage;
use serde_json::{Map, Value};

/// Fill in a hug axis has no size to fill; on the cross axis, stretch says what was meant.
fn fill_in_hug(key: &str, parent: &Value) -> StorageError {
    let mut message = "Fill inside a hug axis is ambiguous and unsupported".to_string();
    let cross = match parent.pointer("/layout/mode").and_then(Value::as_str) {
        Some("column") => "width",
        Some("row") => "height",
        _ => "",
    };
    if key == cross {
        message += &format!("; to size children to the widest one, set the parent's layout.align to \"stretch\" and omit the child's {key}");
    }
    StorageError::validation(key, &message)
}

impl Resolver<'_> {
    pub(super) fn layout_properties(&self, n: &Node, p: &Map<String, Value>) -> Result<()> {
        for key in ["width", "height"] {
            if let Some(v) = p.get(key) {
                if !matches!(v.as_str(), Some("fill" | "hug")) {
                    if v.as_str().and_then(percentage).is_some() {
                        if !self.definite_axis(self.ancestors(&n.id), key, false)? {
                            return Err(StorageError::validation(
                                key,
                                "Percentage requires a definite parent axis",
                            ));
                        }
                    } else if v.is_string() {
                        return Err(StorageError::validation(
                            key,
                            "Use fill, hug or a literal decimal percentage from 0% to 100%",
                        ));
                    } else {
                        number(v, 0., 100_000., key)?;
                    }
                }
                if v == "fill" && n.props.get("component").is_none() {
                    if let Some(parent) = n.parent_id.as_ref().and_then(|id| self.nodes.get(id)) {
                        let props = self.resolve_scoped_props(parent)?;
                        if props.get(key).is_none() || props.get(key).is_some_and(|v| v == "hug") {
                            return Err(fill_in_hug(key, &props));
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
            for (index, rule) in breakpoints::rules(l)?.into_iter().enumerate() {
                validate_layout_rule(rule, index == 0)?;
            }
            if breakpoints::wraps(v)
                && !self.definite_axis(
                    std::iter::once(n).chain(self.ancestors(&n.id)),
                    "width",
                    true,
                )?
            {
                return Err(StorageError::validation(
                    "width",
                    "Wrap requires definite positive width",
                ));
            }
        }
        if let Some(parent) = n.parent_id.as_ref().and_then(|id| self.nodes.get(id)) {
            let parent = self.resolve_scoped_props(parent)?;
            if breakpoints::wraps(&parent["layout"])
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

    fn definite_axis<'a>(
        &self,
        nodes: impl Iterator<Item = &'a Node>,
        key: &str,
        positive: bool,
    ) -> Result<bool> {
        // Node validation checks cycles before calling property validation.
        for node in nodes {
            let props = self.resolve_scoped_props(node)?;
            let size = &props[key];
            if let Some(size) = size.as_f64() {
                return Ok(!positive || size > 0.);
            }
            if size != "fill"
                && !size
                    .as_str()
                    .and_then(percentage)
                    .is_some_and(|v| !positive || v > 0.)
            {
                return Ok(false);
            }
        }
        Ok(false)
    }
}
