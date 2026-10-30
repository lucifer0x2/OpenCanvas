//! Solid-or-gradient fill validation; color-only contexts still use `resolve_value`.
use super::{values::*, Candidate};
use crate::{error::*, model::NodeKind};
use serde_json::Value;

impl Candidate {
    pub(super) fn resolve_paint(&self, value: &Value) -> Result<Value> {
        if let Some(id) = value.get("styleId").and_then(Value::as_str) {
            if value.as_object().is_none_or(|o| o.len() != 1) {
                return Err(StorageError::validation(
                    "fill",
                    "A fill style reference has exactly one ID",
                ));
            }
            let style = self
                .styles
                .get(id)
                .filter(|s| s.kind == "fill")
                .ok_or_else(|| {
                    StorageError::validation(
                        "styleId",
                        "A fill property requires an existing fill style",
                    )
                })?;
            if style.value.get("styleId").is_some() {
                return Err(StorageError::validation(
                    "style.value",
                    "Style aliases are not supported",
                ));
            }
            return self.resolve_paint(&style.value);
        }
        if value.get("type").is_none() && (!value.is_object() || value.get("tokenId").is_some()) {
            return self.resolve_value(value, "color");
        }
        let mut resolved = value.clone();
        if let Some(stops) = resolved.get_mut("stops").and_then(Value::as_array_mut) {
            for (i, stop) in stops.iter_mut().enumerate() {
                if let Some(pair) = stop.as_array_mut().filter(|p| p.len() == 2) {
                    if pair[1].get("styleId").is_some() {
                        return Err(StorageError::validation(
                            &format!("fill.stops[{i}]"),
                            "Stop colors accept color tokens, not fill styles",
                        ));
                    }
                    pair[1] = self.resolve_value(&pair[1], "color").map_err(|mut e| {
                        e.field = Some(format!("fill.stops[{i}]"));
                        e
                    })?;
                }
            }
        }
        Ok(resolved)
    }
}

pub(super) fn paint(v: &Value, kind: &NodeKind) -> Result<()> {
    if v.is_string() {
        return color(v);
    }
    let bad = |field: &str, message: &str| StorageError::validation(field, message);
    let o = v
        .as_object()
        .ok_or_else(|| bad("fill", "Fill must be a solid color or gradient object"))?;
    let linear = match o.get("type").and_then(Value::as_str) {
        Some("linear") => true,
        Some("radial") => false,
        _ => return Err(bad("fill.type", "Gradient type must be linear or radial")),
    };
    if *kind == NodeKind::Text {
        return Err(bad("fill.type", "Gradient fill is not supported on text yet; use a solid color, or put the gradient on a frame behind the text"));
    }
    let allowed: &[&str] = if linear {
        &["type", "angle", "stops"]
    } else {
        &["type", "center", "radius", "stops"]
    };
    if let Some(key) = o.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Err(bad(&format!("fill.{key}"), "Unsupported gradient key"));
    }
    if let Some(angle) = o.get("angle") {
        number(angle, -360., 360., "fill.angle")?;
    }
    if let Some(center) = o.get("center") {
        if !center.as_array().is_some_and(|a| {
            a.len() == 2
                && a.iter()
                    .all(|v| number(v, -10., 10., "fill.center").is_ok())
        }) {
            return Err(bad(
                "fill.center",
                "Radial center is [x, y] as fractions of the node bounds",
            ));
        }
    }
    if let Some(radius) = o.get("radius") {
        let valid = |v: &Value| {
            v.as_f64()
                .is_some_and(|n| n.is_finite() && n > 0. && n <= 100_000.)
        };
        if !(valid(radius)
            || radius
                .as_array()
                .is_some_and(|a| a.len() == 2 && a.iter().all(valid)))
        {
            return Err(bad("fill.radius", "Radial radius is a positive pixel number (circle) or [rx, ry] (ellipse); omit it for the CSS default"));
        }
    }
    let stops = o
        .get("stops")
        .and_then(Value::as_array)
        .filter(|s| (2..=16).contains(&s.len()))
        .ok_or_else(|| {
            bad(
                "fill.stops",
                "Gradient stops must be 2 to 16 [position 0..1, color] pairs",
            )
        })?;
    let mut previous = 0.;
    for (i, stop) in stops.iter().enumerate() {
        let field = format!("fill.stops[{i}]");
        let pair = stop.as_array().filter(|p| p.len() == 2).ok_or_else(|| {
            bad(
                &field,
                "Each stop is [position 0..1, color] with non-decreasing positions",
            )
        })?;
        number(&pair[0], previous, 1., &field)?;
        color(&pair[1]).map_err(|_| {
            bad(
                &field,
                "Stop color must be #RGB, #RGBA, #RRGGBB, #RRGGBBAA or a color token",
            )
        })?;
        previous = pair[0].as_f64().unwrap();
    }
    Ok(())
}
