use opencanvas_storage::Node;
use serde_json::{json, Value};

/// Inspect the complete authored scope, before pagination or property filtering.
pub(super) fn for_scope(nodes: &[(&Node, usize)]) -> Value {
    let Some((frame, _)) = nodes.first() else {
        return json!([]);
    };
    if nodes.iter().any(|(node, depth)| {
        let layout = &node.props["layout"];
        stack(layout)
            || layout["breakpoints"]
                .as_array()
                .is_some_and(|entries| entries.iter().any(|entry| stack(&entry["layout"])))
            || (*depth > 0
                && ["width", "height"].into_iter().any(|key| {
                    node.props[key]
                        .as_str()
                        .is_some_and(|size| size == "fill" || size.ends_with('%'))
                }))
    }) {
        return json!([]);
    }
    let width = &frame.props["width"];
    let height = &frame.props["height"];
    json!([{
        "code": "NO_REFLOW_INFORMATION",
        "frameId": frame.id,
        "authoredSize": {"width": width, "height": height},
        "message": format!(
            "Frame '{}' has authored size {} × {}. All scoped nodes use free layout, explicitly or by default; this context contains no reflow information. State responsive assumptions or ask the user before treating an absolute-positioned implementation as complete.",
            frame.id, width, height
        )
    }])
}

fn stack(layout: &Value) -> bool {
    matches!(layout["mode"].as_str(), Some("row" | "column"))
}

/// Even a scope with responsive rules can produce identical bounds at the requested widths.
pub(super) fn unchanged(nodes: &[(&Node, usize)]) -> Value {
    let frame = nodes[0].0;
    let message = if nodes.iter().any(|(node, _)| stack(&node.props["layout"])) {
        "All descendant bounds are unchanged at the requested viewports. The scope contains stacks, but no reflow was observed for this comparison. State responsive assumptions or ask the user; this does not establish behavior at other sizes."
    } else {
        "All descendant bounds are unchanged at the requested viewports. Check other sizes or ask the user before inferring responsive behavior."
    };
    json!([{
        "code":"NO_REFLOW_INFORMATION", "frameId":frame.id, "reason":"UNCHANGED_BOUNDS",
        "authoredSize":{"width":frame.props["width"],"height":frame.props["height"]},
        "message":message
    }])
}
