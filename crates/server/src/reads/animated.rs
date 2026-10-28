//! `animated` on `get_design_context`: every animation the scope declares, as
//! `{ref: "<nodeId>:<name>", on}` in preorder then array order. Computed over the whole
//! scope before paging, so every page carries the same list; absent when there is none.
use opencanvas_storage::Node;
use serde_json::{json, Value};

pub(super) fn summary(scope: &[(&Node, usize)]) -> Option<Value> {
    let animated: Vec<Value> = scope
        .iter()
        .filter_map(|(node, _)| Some((node, node.props.get("animations")?.as_array()?)))
        .flat_map(|(node, animations)| {
            animations.iter().map(move |a| {
                let on = a.get("on").and_then(Value::as_str).unwrap_or("load");
                json!({"ref": format!("{}:{}", node.id, a["name"].as_str().unwrap_or_default()), "on": on})
            })
        })
        .collect();
    (!animated.is_empty()).then_some(Value::Array(animated))
}
