//! Shape and order of authored layout alternatives.
use crate::{Result, StorageError};
use opencanvas_layout::{valid_breakpoint_width, MAX_BREAKPOINTS};
use serde_json::{Map, Value};

pub(super) fn rules(layout: &Map<String, Value>) -> Result<Vec<&Map<String, Value>>> {
    let mut rules = vec![layout];
    let Some(value) = layout.get("breakpoints") else {
        return Ok(rules);
    };
    let invalid = |message| StorageError::validation("layout.breakpoints", message);
    let entries = value
        .as_array()
        .filter(|entries| entries.len() <= MAX_BREAKPOINTS)
        .ok_or_else(|| invalid("Breakpoints must be an array with at most 8 rules"))?;
    let mut previous = 0;
    for entry in entries {
        let entry = entry
            .as_object()
            .filter(|entry| entry.len() == 2)
            .ok_or_else(|| invalid("Each breakpoint must contain maxWidth and layout"))?;
        let width = entry
            .get("maxWidth")
            .and_then(Value::as_u64)
            .and_then(|width| u32::try_from(width).ok())
            .filter(|width| valid_breakpoint_width(*width, previous))
            .ok_or_else(|| {
                invalid("maxWidth must be a strictly increasing integer in 1..100000")
            })?;
        previous = width;
        rules.push(
            entry
                .get("layout")
                .and_then(Value::as_object)
                .filter(|rule| !rule.contains_key("breakpoints"))
                .ok_or_else(|| {
                    invalid("Each breakpoint requires a full layout without nested breakpoints")
                })?,
        );
    }
    Ok(rules)
}

pub(super) fn wraps(layout: &Value) -> bool {
    let wrap = |rule: &Value| rule["wrap"] == true;
    wrap(layout)
        || layout["breakpoints"]
            .as_array()
            .is_some_and(|entries| entries.iter().any(|entry| wrap(&entry["layout"])))
}
