//! A native export observes only the selected subtree's paint resources.
use serde_json::Value;
use std::collections::HashSet;

pub(super) fn resources(scene: &mut Value, readiness: &Value) -> Value {
    let nodes = scene["nodes"].as_array().unwrap();
    let fonts: HashSet<String> = nodes
        .iter()
        .filter_map(|n| n["fontId"].as_str().map(str::to_owned))
        .collect();
    let assets: HashSet<String> = nodes
        .iter()
        .filter_map(|n| n["assetId"].as_str().map(str::to_owned))
        .collect();
    for (field, ids) in [("fonts", &fonts), ("assets", &assets)] {
        if let Some(list) = scene[field].as_array_mut() {
            list.retain(|r| r["id"].as_str().is_some_and(|id| ids.contains(id)));
        }
    }
    Value::Array(
        readiness
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| {
                r["id"].as_str().is_some_and(|id| {
                    if r["kind"] == "font" {
                        fonts.contains(id)
                    } else {
                        assets.contains(id)
                    }
                })
            })
            .cloned()
            .collect(),
    )
}
