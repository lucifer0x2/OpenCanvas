//! Resource entries preserve full/default metadata without serializing any nodes.
use super::{compact, sanitize_asset};
use crate::error::Result;
use opencanvas_storage::Snapshot;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn entries(
    s: &Snapshot,
    tokens: &BTreeMap<String, Value>,
    compact: Option<&compact::Referenced>,
) -> Result<Vec<(String, Value)>> {
    let mut entries = Vec::new();
    for (field, items) in [
        ("tokens", serde_json::to_value(&s.tokens)?),
        ("styles", serde_json::to_value(&s.styles)?),
        ("assets", serde_json::to_value(&s.assets)?),
        ("fonts", serde_json::to_value(&s.fonts)?),
    ] {
        for item in compact::kept(&items, field, compact) {
            let mut item = sanitize_asset(item.clone());
            if field == "tokens" {
                item["resolvedValue"] = tokens[item["id"].as_str().unwrap()].clone();
            }
            if field == "styles" {
                item["resolvedValue"] = resolve_style(&item["value"], tokens);
            }
            entries.push((field.into(), item));
        }
    }
    Ok(entries)
}
fn resolve_style(value: &Value, tokens: &std::collections::BTreeMap<String, Value>) -> Value {
    if let Some(id) = value["tokenId"].as_str() {
        return tokens.get(id).cloned().unwrap_or(Value::Null);
    }
    match value {
        Value::Object(fields) => fields
            .iter()
            .map(|(k, v)| (k.clone(), resolve_style(v, tokens)))
            .collect(),
        _ => value.clone(),
    }
}
