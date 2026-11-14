//! Assemble the compute boundary without transport URLs or resource I/O.
use crate::{
    check_viewport, resolved_expanded_nodes, Node, NodeKind, Result, Snapshot, StorageError,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneOptions {
    pub frame_id: String,
    pub width: f64,
    pub height: f64,
    pub theme: String,
    pub state: String,
}

/// One theme's resolved and expanded nodes, indexed once so several frames share a pass.
pub struct ResolvedNodes {
    all: Vec<Node>,
    first: HashMap<String, usize>,
    children: HashMap<String, Vec<usize>>,
}

impl ResolvedNodes {
    pub fn new(snapshot: &Snapshot, theme: &str) -> Result<Self> {
        let all = resolved_expanded_nodes(snapshot, theme)?;
        Ok(Self::from_nodes(all))
    }

    /// Index an already resolved scoped closure, retaining stable input order.
    pub fn from_nodes(all: Vec<Node>) -> Self {
        let mut first = HashMap::new();
        let mut children: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, node) in all.iter().enumerate() {
            first.entry(node.id.clone()).or_insert(index);
            if let Some(parent) = &node.parent_id {
                children.entry(parent.clone()).or_default().push(index);
            }
        }
        // Stable: equal sort keys keep their resolved order, as before indexing.
        for list in children.values_mut() {
            list.sort_by_key(|&index| all[index].sort_key);
        }
        Self {
            all,
            first,
            children,
        }
    }

    fn collect(&self, id: &str, out: &mut Vec<Node>) {
        if let Some(&index) = self.first.get(id) {
            out.push(self.all[index].clone());
            for &child in self.children.get(id).into_iter().flatten() {
                self.collect(&self.all[child].id, out);
            }
        }
    }
}

/// Resolve one authored revision into the existing schema-1 layout JSON contract.
/// Resource manifests contain `url: ""`: callers attach their controlled transport
/// URLs after preparation. Keeping JSON also preserves all resolved properties
/// (including interaction states) at the existing layout boundary.
pub fn make_input(snapshot: &Snapshot, options: &SceneOptions) -> Result<Value> {
    make_input_with_scale(snapshot, options, 1.0)
}

/// Prepare a native render request using its output scale for the pixel budget.
pub fn make_input_with_scale(
    snapshot: &Snapshot,
    options: &SceneOptions,
    scale: f64,
) -> Result<Value> {
    check_viewport(options.width, options.height, scale)?;
    let resolved = ResolvedNodes::new(snapshot, &options.theme)?;
    frame_input(snapshot, &resolved, options)
}

/// The layout input of one frame from nodes already resolved for `options.theme`.
/// Callers check the viewport budget for their own output scale first.
pub fn frame_input(
    snapshot: &Snapshot,
    resolved: &ResolvedNodes,
    options: &SceneOptions,
) -> Result<Value> {
    if !resolved
        .first
        .get(&options.frame_id)
        .is_some_and(|&index| resolved.all[index].kind == NodeKind::Frame)
    {
        return Err(invalid("frameId must identify a frame"));
    }
    let mut nodes = Vec::new();
    resolved.collect(&options.frame_id, &mut nodes);
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
    // A hug-height scene root ignores the viewport height; one value keeps inputs equal.
    let height = if root_hugs_height(&nodes[0].props, options.width) {
        1.0
    } else {
        options.height
    };
    Ok(
        json!({"schemaVersion":1,"projectId":snapshot.project_id,"revision":snapshot.revision,
        "frameId":options.frame_id,"viewport":{"width":options.width,"height":height},
        "theme":options.theme,"state":options.state,"nodes":nodes,"fonts":fonts,"assets":assets}),
    )
}

/// Whether a scene root of these resolved props takes its content height at `width`
/// (`height:"hug"` under a row/column rule) instead of the viewport height.
pub fn root_hugs_height(props: &Value, width: f64) -> bool {
    let sizing = json!({"height":props["height"],"layout":props["layout"]});
    serde_json::from_value(sizing)
        .is_ok_and(|props| opencanvas_layout::root_hugs_height(&props, width))
}

fn invalid(message: &str) -> StorageError {
    StorageError::new("INVALID_REQUEST", message)
}
