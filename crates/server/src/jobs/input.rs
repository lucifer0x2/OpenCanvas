use crate::{
    error::{ApiError, Result},
    jobs::SceneRequest,
};
use opencanvas_storage::{resolved_expanded_nodes, Node, NodeKind, Snapshot};
use serde_json::{json, Value};
use std::collections::HashSet;

pub fn make_input(snapshot: &Snapshot, request: &SceneRequest, base: &str) -> Result<Value> {
    let all = resolved_expanded_nodes(snapshot, &request.theme)?;
    if !all
        .iter()
        .any(|n| n.id == request.frame_id && n.kind == NodeKind::Frame)
    {
        return Err(ApiError::invalid("frameId must identify a frame"));
    }
    let mut nodes = Vec::new();
    collect(&all, &request.frame_id, &mut nodes);
    // A frame scene is a standalone subtree; its authored page is outside this input.
    nodes[0].parent_id = None;
    if nodes.len() > 2048 {
        return Err(ApiError::invalid("Frame exceeds 2048 render nodes"));
    }
    let fonts_used: HashSet<_> = nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Text)
        .map(|n| n.props["fontId"].as_str().unwrap_or("font_bundled"))
        .collect();
    let assets_used: HashSet<_> = nodes
        .iter()
        .filter_map(|n| n.props["assetId"].as_str())
        .collect();
    let fonts:Vec<_> = snapshot.fonts.iter().filter(|f| fonts_used.contains(f.id.as_str())).map(|f| {
        let url = if let Some(asset) = &f.asset_id { format!("{base}/api/v1/projects/{}/assets/{asset}",snapshot.project_id) } else { format!("{base}/api/v1/fonts/font_bundled") };
        json!({"id":f.id,"family":f.family,"hash":f.content_hash,"url":url,"weight":f.weight,"style":f.style})
    }).collect();
    let assets:Vec<_> = snapshot.assets.iter().filter(|a| assets_used.contains(a.id.as_str())).map(|a|
        json!({"id":a.id,"hash":a.content_hash,"url":format!("{base}/api/v1/projects/{}/assets/{}",snapshot.project_id,a.id),"mediaType":a.media_type})).collect();
    Ok(
        json!({"schemaVersion":1,"projectId":snapshot.project_id,"revision":snapshot.revision,
        "frameId":request.frame_id,"viewport":{"width":request.width,"height":request.height},
        "theme":request.theme,"state":request.state,"nodes":nodes,"fonts":fonts,"assets":assets}),
    )
}
fn collect(all: &[Node], id: &str, out: &mut Vec<Node>) {
    if let Some(node) = all.iter().find(|n| n.id == id) {
        out.push(node.clone());
        let mut children: Vec<_> = all
            .iter()
            .filter(|n| n.parent_id.as_deref() == Some(id))
            .collect();
        children.sort_by_key(|n| n.sort_key);
        for child in children {
            collect(all, &child.id, out);
        }
    }
}
/// Rejects a request outside the render budget or the supported interaction state.
pub fn check_request(snapshot: &Snapshot, request: &SceneRequest) -> Result<()> {
    if snapshot.revision != request.revision {
        return Err(ApiError::new(
            "REVISION_UNAVAILABLE",
            "Requested authored revision is unavailable",
        ));
    }
    if !request.width.is_finite()
        || !request.height.is_finite()
        || request.width <= 0.0
        || request.height <= 0.0
        || request.width > 100000.0
        || request.height > 100000.0
        || !request.scale.is_finite()
        || request.scale <= 0.0
        || request.scale > 4.0
        || request.width * request.height * request.scale * request.scale > 16_000_000.0
    {
        return Err(ApiError::invalid(
            "Viewport or scale exceeds the render budget",
        ));
    }
    if request.state != "default" {
        return Err(ApiError::invalid(
            "Only default interaction state is supported",
        ));
    }
    Ok(())
}
