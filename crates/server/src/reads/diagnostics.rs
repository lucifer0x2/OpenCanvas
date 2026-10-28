use opencanvas_storage::Node;
use serde_json::{json, Value};

/// Inspect the complete authored scope, before pagination or property filtering.
pub(super) fn for_scope(nodes: &[(&Node, usize)]) -> Value {
    let Some((frame, _)) = nodes.first() else {
        return json!([]);
    };
    if nodes.iter().any(|(node, _)| {
        matches!(
            node.props["layout"]["mode"].as_str(),
            Some("row" | "column")
        )
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

/// Stacks may be fixed or constrained: observed equality does not imply all-free authoring.
pub(super) fn unchanged(frame: &Node) -> Value {
    json!([{
        "code":"NO_REFLOW_INFORMATION", "frameId":frame.id, "reason":"UNCHANGED_BOUNDS",
        "authoredSize":{"width":frame.props["width"],"height":frame.props["height"]},
        "message":"All descendant bounds are unchanged at the requested viewports. The scope contains stacks, but no reflow was observed for this comparison. State responsive assumptions or ask the user; this does not establish behavior at other sizes."
    }])
}
