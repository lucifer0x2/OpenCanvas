//! Paint interaction overrides after layout/measurement, including inside stroke changes.
//! A variant never changes bounds, line wrapping, or descendant positions.
use serde_json::{json, Value};
use std::collections::HashMap;

pub(super) fn apply(scene: &mut Value, input: &Value) {
    let state = input["state"].as_str().unwrap_or("default");
    if state == "default" {
        return;
    }
    let variants: HashMap<_, _> = input["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|node| {
            Some((
                node["id"].as_str()?,
                node["props"]["states"][state].as_object()?,
            ))
        })
        .collect();
    if variants.is_empty() {
        scene["diagnostics"]
            .as_array_mut()
            .unwrap()
            .push(json!("STATE_NOT_DECLARED"));
    }
    for node in scene["nodes"].as_array_mut().unwrap() {
        let Some(overrides) = variants.get(node["id"].as_str().unwrap()) else {
            continue;
        };
        for (key, value) in *overrides {
            if key == "stroke" {
                let width = sides(value.get("width").unwrap_or(&json!(1)));
                if width.iter().all(|v| v.as_f64() == Some(0.)) {
                    node.as_object_mut().unwrap().remove("stroke");
                } else {
                    node["stroke"] = json!({"color":sides(&value["color"]),"width":width,
                        "align":value["align"].as_str().unwrap_or("inside")});
                }
            } else {
                node[key] = value.clone();
            }
        }
    }
}

fn sides(value: &Value) -> Vec<Value> {
    value
        .as_array()
        .cloned()
        .unwrap_or_else(|| vec![value.clone(); 4])
}
