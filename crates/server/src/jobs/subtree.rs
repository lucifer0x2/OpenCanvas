//! Cuts one node's subtree out of a computed frame scene, as a scene of its own: the node at the
//! origin, its viewport equal to its layout bounds, and no ancestors, so ancestor clip and
//! opacity do not apply. Text line positions are node-relative and need no shift.
use crate::error::{ApiError, Result};
use serde_json::{json, Value};
use std::collections::HashSet;

pub fn scene(frame: &Value, node_id: &str) -> Result<Value> {
    let nodes = frame["nodes"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let root = nodes.iter().find(|n| n["id"] == node_id).ok_or_else(|| {
        ApiError::new("NODE_NOT_FOUND", "Node is not in the computed frame scene")
    })?;
    let bounds = &root["bounds"];
    let (x, y) = (bounds["x"].as_f64(), bounds["y"].as_f64());
    let (width, height) = (bounds["width"].as_f64(), bounds["height"].as_f64());
    let (Some(x), Some(y), Some(width), Some(height)) = (x, y, width, height) else {
        return Err(ApiError::new("WORKER_FAILED", "Scene node has no bounds"));
    };
    if width <= 0.0 || height <= 0.0 {
        return Err(ApiError::new(
            "EMPTY_BOUNDS",
            "Node has zero width or height in its frame's layout",
        ));
    }
    // Scene nodes are in paint order, parents before children.
    let mut kept = HashSet::from([node_id.to_owned()]);
    let mut out = Vec::new();
    for node in nodes {
        let id = node["id"].as_str().unwrap_or_default();
        let inside = id == node_id
            || node["parentId"]
                .as_str()
                .is_some_and(|parent| kept.contains(parent));
        if !inside {
            continue;
        }
        kept.insert(id.to_owned());
        let mut node = node.clone();
        if id == node_id {
            node["parentId"] = Value::Null;
        }
        let b = &mut node["bounds"];
        b["x"] = json!(b["x"].as_f64().unwrap_or_default() - x);
        b["y"] = json!(b["y"].as_f64().unwrap_or_default() - y);
        out.push(node);
    }
    let mut scene = frame.clone();
    scene["nodes"] = Value::Array(out);
    scene["provenance"]["viewport"] = json!({"width":width,"height":height});
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use super::scene;
    use serde_json::json;

    fn node(id: &str, parent: Option<&str>, x: f64, y: f64, w: f64, h: f64) -> serde_json::Value {
        json!({"id":id,"parentId":parent,"bounds":{"x":x,"y":y,"width":w,"height":h},"lines":[]})
    }

    #[test]
    fn keeps_descendants_shifted_to_the_node_origin() {
        let frame = json!({"provenance":{"viewport":{"width":400,"height":300}},"nodes":[
            node("frame", None, 0.0, 0.0, 400.0, 300.0),
            node("card", Some("frame"), 20.0, 30.0, 133.5, 64.0),
            node("icon", Some("card"), 28.0, 40.0, 16.0, 16.0),
            node("other", Some("frame"), 200.0, 30.0, 50.0, 50.0),
        ]});
        let sub = scene(&frame, "card").unwrap();
        let ids: Vec<_> = sub["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["id"].clone())
            .collect();
        assert_eq!(ids, [json!("card"), json!("icon")]);
        assert_eq!(sub["nodes"][0]["parentId"], json!(null));
        assert_eq!(sub["nodes"][0]["bounds"]["x"], json!(0.0));
        assert_eq!(sub["nodes"][1]["bounds"]["x"], json!(8.0));
        assert_eq!(sub["nodes"][1]["bounds"]["y"], json!(10.0));
        assert_eq!(
            sub["provenance"]["viewport"],
            json!({"width":133.5,"height":64.0})
        );
    }

    #[test]
    fn zero_size_node_is_empty_bounds() {
        let frame = json!({"provenance":{},"nodes":[node("line", None, 0.0, 0.0, 0.0, 20.0)]});
        assert_eq!(scene(&frame, "line").unwrap_err().0.code, "EMPTY_BOUNDS");
    }
}
