//! Assemble the compute boundary without transport URLs or resource I/O.
use crate::{resolved_expanded_nodes, Node, NodeKind, Result, Snapshot, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneOptions {
    pub frame_id: String,
    pub width: f64,
    pub height: f64,
    pub theme: String,
    pub state: String,
}

/// Resolve one authored revision into the existing schema-1 layout JSON contract.
/// Resource manifests contain `url: ""`: callers attach their controlled transport
/// URLs after preparation. Keeping JSON also preserves all resolved properties
/// (including interaction states) at the existing layout boundary.
pub fn make_input(snapshot: &Snapshot, options: &SceneOptions) -> Result<Value> {
    let all = resolved_expanded_nodes(snapshot, &options.theme)?;
    if !all
        .iter()
        .any(|n| n.id == options.frame_id && n.kind == NodeKind::Frame)
    {
        return Err(invalid("frameId must identify a frame"));
    }
    let mut nodes = Vec::new();
    collect(&all, &options.frame_id, &mut nodes);
    // A frame scene is standalone; its authored parent is outside the input.
    nodes[0].parent_id = None;
    if nodes.len() > 2048 {
        return Err(invalid("Frame exceeds 2048 render nodes"));
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
    let fonts: Vec<_> = snapshot
        .fonts
        .iter()
        .filter(|f| fonts_used.contains(f.id.as_str()))
        .map(|f| {
            json!({"id":f.id,"family":f.family,"hash":f.content_hash,
            "url":"","weight":f.weight,"style":f.style})
        })
        .collect();
    let assets: Vec<_> = snapshot
        .assets
        .iter()
        .filter(|a| assets_used.contains(a.id.as_str()))
        .map(|a| json!({"id":a.id,"hash":a.content_hash,"url":"","mediaType":a.media_type}))
        .collect();
    Ok(
        json!({"schemaVersion":1,"projectId":snapshot.project_id,"revision":snapshot.revision,
        "frameId":options.frame_id,"viewport":{"width":options.width,"height":options.height},
        "theme":options.theme,"state":options.state,"nodes":nodes,"fonts":fonts,"assets":assets}),
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

fn invalid(message: &str) -> StorageError {
    StorageError::new("INVALID_REQUEST", message)
}
