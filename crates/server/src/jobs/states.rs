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
                (
                    node["props"]["states"][state].as_object()?,
                    node["props"]["visible"] == false,
                ),
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
        let Some((overrides, hidden)) = variants.get(node["id"].as_str().unwrap()) else {
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
            } else if key == "radius" && value.is_array() {
                node[key] = json!(sides(value));
            } else if key == "fill" && value.is_object() {
                // Reuse the layout paint defaults and normalization for interaction variants.
                let mut paint = value.clone();
                let props = paint.as_object_mut().unwrap();
                if value["type"] == "linear" {
                    props.entry("angle").or_insert(json!(180));
                }
                if value["type"] == "radial" {
                    props.entry("center").or_insert(json!([0.5, 0.5]));
                    props.entry("radius").or_insert(Value::Null);
                }
                node[key] = paint;
            } else {
                node[key] = value.clone();
            }
        }
        // Expansion encodes visible:false as opacity zero; a variant must not unhide it.
        if *hidden {
            node["opacity"] = json!(0);
        }
    }
}

fn sides(value: &Value) -> Vec<Value> {
    match value.as_array().map(Vec::as_slice) {
        Some([a]) => vec![a.clone(); 4],
        Some([a, b]) => vec![a.clone(), b.clone(), a.clone(), b.clone()],
        Some([a, b, c]) => vec![a.clone(), b.clone(), c.clone(), b.clone()],
        Some(v) => v.to_vec(),
        None => vec![value.clone(); 4],
    }
}
