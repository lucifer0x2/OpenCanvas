//! What exporting one node means: its root frame laid out at authored size, and its settings
//! (the call's, else the node's `exportSettings`, else one PNG at 1x) with file names.
use crate::{
    error::{ApiError, Result},
    jobs::{ExportSetting, ExportSpec, SceneRequest},
};
use opencanvas_storage::{export_format, export_suffix, Node, NodeKind, Snapshot};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

const MAX_NAME_BYTES: usize = 100;

/// Authored and theme-resolved nodes of one revision, by ID.
pub struct Nodes<'a> {
    pub authored: HashMap<&'a str, &'a Node>,
    pub resolved: HashMap<&'a str, &'a Node>,
}

impl<'a> Nodes<'a> {
    pub fn new(snapshot: &'a Snapshot, resolved: &'a [Node]) -> Self {
        let by_id = |nodes: &'a [Node]| nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        Self {
            authored: by_id(&snapshot.nodes),
            resolved: by_id(resolved),
        }
    }
}

pub fn resolve(
    nodes: &Nodes,
    snapshot: &Snapshot,
    node_id: &str,
    theme: &str,
    settings: Option<&Value>,
) -> Result<(SceneRequest, ExportSpec)> {
    let node = *nodes
        .authored
        .get(node_id)
        .ok_or_else(|| ApiError::new("NODE_NOT_FOUND", "Node does not exist at this revision"))?;
    let root = root_frame(nodes, node)?;
    let size = |key: &str| nodes.resolved.get(root.id.as_str())?.props[key].as_f64();
    let (Some(width), Some(height)) = (size("width"), size("height")) else {
        return Err(unsupported("Root frame has no fixed width and height"));
    };
    let request = SceneRequest {
        frame_id: root.id.clone(),
        revision: snapshot.revision,
        width,
        height,
        theme: theme.into(),
        state: "default".into(),
        scale: 1.0,
    };
    let authored = node.props.get("exportSettings");
    let items = settings
        .or(authored)
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .cloned()
        .unwrap_or_else(|| vec![json!({})]);
    let name = file_stem(node);
    let settings = items
        .iter()
        .map(|item| setting(item.as_object().unwrap_or(&Map::new()), &name))
        .collect();
    let spec = ExportSpec {
        node_id: node.id.clone(),
        settings,
    };
    Ok((request, spec))
}

/// The page's child that contains the node, which must be a frame.
fn root_frame<'a>(nodes: &Nodes<'a>, node: &'a Node) -> Result<&'a Node> {
    if node.kind == NodeKind::Page {
        return Err(unsupported(
            "Pages cannot be exported; export a frame or a node inside one",
        ));
    }
    let mut current = node;
    for _ in 0..=nodes.authored.len() {
        let parent = current
            .parent_id
            .as_deref()
            .and_then(|id| nodes.authored.get(id).copied())
            .ok_or_else(|| unsupported("Node is not inside a root frame"))?;
        if parent.kind == NodeKind::Page {
            return match current.kind {
                NodeKind::Frame => Ok(current),
                _ => Err(unsupported("Node is not inside a root frame")),
            };
        }
        current = parent;
    }
    Err(unsupported("Node is not inside a root frame"))
}

fn setting(item: &Map<String, Value>, stem: &str) -> ExportSetting {
    let format = export_format(item).to_owned();
    let file_name = format!("{stem}{}.{format}", export_suffix(item));
    let scale = item.get("scale").and_then(Value::as_f64);
    let (width, height) = (item.get("width"), item.get("height"));
    let raster = format != "svg" && width.is_none() && height.is_none();
    ExportSetting {
        scale: scale.or(raster.then_some(1.0)),
        width: width.and_then(Value::as_u64),
        height: height.and_then(Value::as_u64),
        file_name,
        svg_id_attribute: item.get("svgIdAttribute") == Some(&Value::Bool(true)),
        format,
    }
}

/// The node's name (else its ID) with path and shell-reserved characters replaced.
fn file_stem(node: &Node) -> String {
    let name = node.props["name"]
        .as_str()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or(&node.id);
    let mut stem = String::new();
    for c in name.chars() {
        let c = if c.is_control() || "/\\:*?\"<>|".contains(c) {
            '_'
        } else {
            c
        };
        if stem.len() + c.len_utf8() > MAX_NAME_BYTES {
            break;
        }
        stem.push(c);
    }
    stem
}

fn unsupported(message: &str) -> ApiError {
    ApiError::new("EXPORT_UNSUPPORTED", message)
}
