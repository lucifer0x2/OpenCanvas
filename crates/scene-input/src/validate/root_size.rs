//! Root frame size: a numeric width, and a numeric height or `hug`. A hug root's height is
//! its stacked content, so every layout rule (base and breakpoints) must be a row or column;
//! free children never size their parent.
use super::breakpoints;
use crate::error::*;
use serde_json::{Map, Value};

/// Checks resolved props of a frame placed directly under a page that is not a component.
pub(super) fn check_root_size(p: &Map<String, Value>) -> Result<()> {
    if !p.get("width").is_some_and(Value::is_number) {
        return Err(StorageError::validation(
            "width",
            "Root frames require a numeric width",
        ));
    }
    match p.get("height") {
        Some(height) if height.is_number() => Ok(()),
        Some(height) if height == "hug" => {
            let stacks = match p.get("layout").and_then(Value::as_object) {
                Some(layout) => breakpoints::rules(layout)?
                    .into_iter()
                    .all(|rule| matches!(rule["mode"].as_str(), Some("row" | "column"))),
                None => false,
            };
            if stacks {
                Ok(())
            } else {
                Err(StorageError::validation(
                    "height",
                    "Root height hug requires a row or column layout in every rule",
                ))
            }
        }
        _ => Err(StorageError::validation(
            "height",
            "Root frames require a numeric height or \"hug\"",
        )),
    }
}
