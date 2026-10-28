//! Per-operation shape checks of navigation props (`links`, `flowStart`); destinations and
//! flow names are checked once per batch in `navigation.rs`.
use super::{values::valid_id, Candidate};
use crate::{
    error::*,
    links::{Link, LINK_ACTIONS, LINK_KEYS, LINK_TRIGGERS, MAX_FLOW_NAME},
    model::*,
};
use serde_json::Value;
use std::collections::BTreeMap;

/// The shape of `props.links`: an array of {to?, on?, action?}, one link per trigger.
pub(super) fn check_links(value: &Value) -> Result<()> {
    let items = value.as_array().ok_or_else(|| {
        StorageError::validation("links", "links must be an array of {to, on, action}")
    })?;
    let mut triggers: BTreeMap<String, usize> = BTreeMap::new();
    for (i, item) in items.iter().enumerate() {
        let field = |key: &str| format!("links[{i}].{key}");
        let link = item.as_object().ok_or_else(|| {
            StorageError::validation(&format!("links[{i}]"), "Each link must be an object")
        })?;
        if let Some(key) = link.keys().find(|k| !LINK_KEYS.contains(&k.as_str())) {
            let mut e = StorageError::validation(
                &field(key),
                &format!("Unsupported link key '{key}'; link keys are to, on, action"),
            );
            e.node_context().suggestion = link_key_suggestion(key).map(Into::into);
            return Err(e);
        }
        let on = enumerated(link.get("on"), &LINK_TRIGGERS, &field("on"), || {
            "on must be \"click\" or \"hover\"".into()
        })?;
        let action = enumerated(link.get("action"), &LINK_ACTIONS, &field("action"), || {
            "action must be navigate, overlay, back or close".into()
        })?;
        match (Link::needs_target(action), link.get("to")) {
            (true, None) => {
                return Err(StorageError::validation(
                    &field("to"),
                    "navigate and overlay need to: a root frame ID",
                ))
            }
            (false, Some(_)) => {
                return Err(StorageError::validation(
                    &field("to"),
                    "back and close take no to",
                ))
            }
            (true, Some(to)) if !to.as_str().is_some_and(valid_id) => {
                return Err(StorageError::validation(
                    &field("to"),
                    "to must be a root frame ID of 1–128 ASCII letters, digits, underscores or hyphens",
                ))
            }
            _ => {}
        }
        if on == "hover" && action != "overlay" {
            return Err(StorageError::validation(
                &field("action"),
                "hover opens an overlay while hovered; use on:\"click\" to navigate",
            ));
        }
        if let Some(first) = triggers.insert(on.into(), i) {
            return Err(StorageError::validation(
                &field("on"),
                &format!("One link per trigger: links[{first}] already uses {on}"),
            ));
        }
    }
    Ok(())
}

/// A string from `allowed`, or the first (default) when absent.
fn enumerated<'a>(
    value: Option<&'a Value>,
    allowed: &[&'static str],
    field: &str,
    message: impl Fn() -> String,
) -> Result<&'a str> {
    match value {
        None => Ok(allowed[0]),
        Some(v) => v
            .as_str()
            .filter(|s| allowed.contains(s))
            .ok_or_else(|| StorageError::validation(field, &message())),
    }
}

fn link_key_suggestion(key: &str) -> Option<&'static str> {
    match key.to_ascii_lowercase().as_str() {
        "trigger" | "event" | "when" => Some("on"),
        "destination" | "destinationid" | "target" | "frame" | "frameid" | "href" => Some("to"),
        "type" | "navigation" | "kind" => Some("action"),
        _ => None,
    }
}

impl Candidate {
    /// `flowStart` is a flow name of 1–64 bytes on a root frame (a direct child of a page).
    pub(super) fn check_flow_start(&self, n: &Node, value: &Value) -> Result<()> {
        if value
            .as_str()
            .is_none_or(|s| s.is_empty() || s.len() > MAX_FLOW_NAME)
        {
            return Err(StorageError::validation(
                "flowStart",
                "flowStart must be a flow name of 1–64 bytes",
            ));
        }
        if !self.is_root_frame(&n.id) {
            return Err(StorageError::validation(
                "flowStart",
                "flowStart marks a root frame (a direct child of a page)",
            ));
        }
        Ok(())
    }
}
