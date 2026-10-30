//! The authored `layout` object: keys, padding forms, stack alignment and token refs.
use super::{values::number, Candidate};
use crate::error::*;
use serde_json::{Map, Value};

const ALIGNS: [&str; 4] = ["start", "center", "end", "stretch"];
const JUSTIFIES: [&str; 4] = ["start", "center", "end", "space-between"];
const PADDING_HELP: &str = "Padding is a number or 1–4 numbers like CSS: [all], [vertical, horizontal], [top, horizontal, bottom] or [top, right, bottom, left]; each 0..100000 or a dimension token";

/// Checks every key of a resolved `layout` object except the wrap width chain.
pub(super) fn validate_layout_rule(l: &Map<String, Value>) -> Result<()> {
    let mode = l.get("mode").and_then(Value::as_str);
    if !matches!(mode, Some("row" | "column" | "free")) {
        let mut message = "Only row, column and free layout are supported".to_string();
        if !l.contains_key("mode") {
            message += "; layout.mode is missing: update_node replaces the whole layout object; resend mode with the other layout keys";
        }
        return Err(StorageError::validation("layout.mode", &message));
    }
    for (key, value) in l {
        match key.as_str() {
            "mode" => {}
            "padding" => padding(value)?,
            "gap" => number(value, 0., 100_000., key)?,
            "wrap" if value.is_boolean() => {}
            "wrap" => {
                return Err(StorageError::validation(
                    "layout.wrap",
                    "Wrap must be a literal boolean",
                ))
            }
            "align" => keyword("align", value, &ALIGNS)?,
            "justify" => keyword("justify", value, &JUSTIFIES)?,
            _ => return Err(unknown_key(key)),
        }
    }
    for key in ["align", "justify"] {
        if mode == Some("free") && l.contains_key(key) {
            return Err(StorageError::validation(
                &format!("layout.{key}"),
                &format!("layout.{key} requires mode \"row\" or \"column\"; free children use x/y"),
            ));
        }
    }
    Ok(())
}

fn padding(value: &Value) -> Result<()> {
    let ok = |v: &Value| v.as_f64().is_some_and(|n| (0.0..=100_000.0).contains(&n));
    let valid = super::quad::quad(value, ok);
    if valid {
        Ok(())
    } else {
        Err(StorageError::validation("layout.padding", PADDING_HELP))
    }
}

fn keyword(key: &str, value: &Value, allowed: &[&str]) -> Result<()> {
    let text = value.as_str().unwrap_or_default();
    if allowed.contains(&text) {
        return Ok(());
    }
    let list = allowed
        .iter()
        .map(|v| format!("\"{v}\""))
        .collect::<Vec<_>>();
    let (head, last) = list.split_at(list.len() - 1);
    let message = format!("layout.{key} must be {} or {}", head.join(", "), last[0]);
    let mut e = StorageError::validation(&format!("layout.{key}"), &message);
    e.node_context().suggestion = suggest_keyword(text, allowed).map(Into::into);
    Err(e)
}

/// CSS, Figma and direction spellings of a supported alignment keyword.
fn suggest_keyword(text: &str, allowed: &[&str]) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase().replace('_', "-");
    let plain = lower.strip_prefix("flex-").unwrap_or(&lower);
    let word = match plain {
        "min" | "left" | "top" => "start",
        "max" | "right" | "bottom" => "end",
        "middle" => "center",
        "between" => "space-between",
        other => other,
    };
    [ALIGNS, JUSTIFIES]
        .into_iter()
        .flatten()
        .find(|v| *v == word && allowed.contains(v))
}

fn unknown_key(key: &str) -> StorageError {
    let lower = key.to_ascii_lowercase();
    let suggestion = match lower.as_str() {
        k if k.starts_with("padding") => Some("padding"),
        k if k.contains("justify") || k.starts_with("primaryaxis") || k.starts_with("mainaxis") => {
            Some("justify")
        }
        k if k.contains("align") => Some("align"),
        "itemspacing" | "spacing" | "rowgap" | "columngap" | "gapx" | "gapy" => Some("gap"),
        "direction" | "flexdirection" | "layoutmode" => Some("mode"),
        "flexwrap" | "layoutwrap" => Some("wrap"),
        _ => None,
    };
    let mut message = format!("Unsupported layout property '{key}'");
    if let Some(s) = suggestion {
        message += &format!("; did you mean '{s}'?");
    }
    if suggestion == Some("padding") {
        message += " Use padding:[top, right, bottom, left].";
    }
    let mut e = StorageError::validation(&format!("layout.{key}"), &message);
    e.node_context().suggestion = suggestion.map(Into::into);
    e
}

impl Candidate {
    /// Resolves dimension tokens in `padding` (object or list items) and `gap`.
    pub(super) fn resolve_layout_refs(&self, layout: &mut Value) -> Result<()> {
        let Some(layout) = layout.as_object_mut() else {
            return Ok(());
        };
        for key in ["padding", "gap"] {
            if key == "padding" {
                if let Some(v) = layout.get_mut(key) {
                    self.resolve_quad(v, "dimension", "layout.padding")?;
                }
                continue;
            }
            match layout.get_mut(key) {
                Some(x) if x.is_object() => *x = self.resolve_value(x, "dimension")?,
                Some(Value::Array(sides)) if key == "padding" => {
                    for side in sides.iter_mut().filter(|v| v.is_object()) {
                        *side = self.resolve_value(side, "dimension").map_err(|_| {
                            StorageError::validation("layout.padding", PADDING_HELP)
                        })?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
