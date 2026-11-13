//! Attribute key names: OCD shorthands and the catalog keys they write, which keys keep bare
//! values as strings, and the unknown-key error with the most likely intended key.
use super::error::OcdError;
use super::lexer::Token;
use opencanvas_storage::{prop_spec, suggest_prop};

const RENAMES: [(&str, &str); 6] = [
    ("w", "width"),
    ("h", "height"),
    ("r", "radius"),
    ("font", "fontId"),
    ("family", "fontFamily"),
    ("weight", "fontWeight"),
];
/// Shorthands for one field of `layout`.
const LAYOUT: [(&str, &str); 4] = [
    ("p", "padding"),
    ("gap", "gap"),
    ("align", "align"),
    ("justify", "justify"),
];
/// Keys whose bare value stays a string even when it looks like a number.
const LITERAL: [&str; 9] = [
    "text",
    "name",
    "theme",
    "fontId",
    "fontFamily",
    "styleId",
    "assetId",
    "componentId",
    "flowStart",
];
/// Names authors try for OCD shorthands; catalog aliases are covered by `suggest_prop`.
const ALIASES: [(&str, &str); 4] = [
    ("padding", "p"),
    ("spacing", "gap"),
    ("alignItems", "align"),
    ("justifyContent", "justify"),
];

/// The catalog key an attribute key writes, or `None` when it is unknown.
pub fn effective_key(key: &str) -> Option<&'static str> {
    if let Some((_, to)) = RENAMES.iter().find(|(from, _)| *from == key) {
        return Some(to);
    }
    if LAYOUT.iter().any(|(from, _)| *from == key) {
        return Some("layout");
    }
    match key {
        "size" => Some("fontSize"),
        _ => prop_spec(key).map(|spec| spec.key),
    }
}

/// The `layout` field a layout shorthand writes.
pub fn layout_field(key: &str) -> Option<&'static str> {
    LAYOUT
        .iter()
        .find(|(from, _)| *from == key)
        .map(|(_, to)| *to)
}

pub fn is_literal(key: &str) -> bool {
    LITERAL.contains(&key)
}

pub fn unknown(key: &Token) -> OcdError {
    let k = key.text.as_str();
    let alias = ALIASES
        .iter()
        .find(|(from, _)| from.eq_ignore_ascii_case(k))
        .map(|(_, to)| *to);
    key.error(format!("Unknown key `{k}`"))
        .suggest(alias.or_else(|| suggest_prop(k)))
}
