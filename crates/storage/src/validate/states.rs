//! Flat, paint-only interaction variants. Geometry always uses resting properties.
use super::{stroke::validate_stroke, values::*, Candidate};
use crate::{error::*, model::NodeKind, props_catalog::PropScope};
use serde_json::Value;

pub const STATES: [&str; 4] = ["hover", "pressed", "focus", "disabled"];

fn keys(kind: &NodeKind) -> Vec<&'static str> {
    let mut keys = vec!["fill", "opacity", "radius"];
    if PropScope::Box.allows(kind) {
        keys.push("stroke");
    }
    keys
}

fn unknown(field: &str, allowed: &[&str]) -> StorageError {
    let mut e = StorageError::validation(field, "Unsupported interaction state or visual property");
    e.node_context().allowed_keys = Some(allowed.iter().map(|k| (*k).into()).collect());
    e
}

fn at(mut e: StorageError, prefix: &str, key: &str) -> StorageError {
    let suffix = e.field.as_deref().unwrap_or(key);
    e.field = Some(if suffix.starts_with("stroke.") {
        format!("{prefix}.{suffix}")
    } else {
        format!("{prefix}.{key}")
    });
    e
}

impl Candidate {
    /// Shape-check before traversing refs so unknown nested keys never hide behind ref errors.
    pub(super) fn check_states(&self, kind: &NodeKind, states: &Value) -> Result<()> {
        let states = states
            .as_object()
            .ok_or_else(|| StorageError::validation("props.states", "States must be an object"))?;
        let allowed = keys(kind);
        for (name, state) in states {
            let prefix = format!("props.states.{name}");
            if !STATES.contains(&name.as_str()) {
                return Err(unknown(&prefix, &STATES));
            }
            let state = state.as_object().ok_or_else(|| {
                StorageError::validation(&prefix, "State must be a flat visual property object")
            })?;
            for (key, value) in state {
                if !allowed.contains(&key.as_str()) {
                    return Err(unknown(&format!("{prefix}.{key}"), &allowed));
                }
                // Refs are checked by resolve_states; literals share the base prop checks.
                if value.is_object() && key != "stroke" {
                    continue;
                }
                let result = match key.as_str() {
                    "fill" => color(value),
                    "opacity" => number(value, 0., 1., key),
                    "radius" => number(value, 0., 100_000., key),
                    "stroke" => validate_stroke(value),
                    _ => unreachable!(),
                };
                result.map_err(|e| at(e, &prefix, key))?;
            }
        }
        Ok(())
    }

    pub(super) fn resolve_states(&self, kind: &NodeKind, value: &mut Value) -> Result<()> {
        // Check only shape/keys here: stroke refs must resolve before literal validation.
        let Some(states) = value.as_object_mut() else {
            return self.check_states(kind, value);
        };
        let allowed = keys(kind);
        for (name, state) in states {
            let prefix = format!("props.states.{name}");
            if !STATES.contains(&name.as_str()) {
                return Err(unknown(&prefix, &STATES));
            }
            let state = state.as_object_mut().ok_or_else(|| {
                StorageError::validation(&prefix, "State must be a flat visual property object")
            })?;
            for (key, v) in state {
                if !allowed.contains(&key.as_str()) {
                    return Err(unknown(&format!("{prefix}.{key}"), &allowed));
                }
                let result = match key.as_str() {
                    "stroke" => self.resolve_stroke(v),
                    key => self
                        .resolve_value(
                            v,
                            match key {
                                "fill" => "color",
                                "radius" => "dimension",
                                _ => "number",
                            },
                        )
                        .map(|resolved| *v = resolved),
                };
                result.map_err(|e| at(e, &prefix, key))?;
            }
        }
        self.check_states(kind, value)
    }
}
