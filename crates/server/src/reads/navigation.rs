//! Navigation authored as `props.links` and `props.flowStart`, read as edges between screens.
//! `get_design_context` summarizes one frame; a project-wide map can reuse `edges`.
use super::index::TreeIndex;
use opencanvas_storage::{flow_start, node_links, Link, Node};
use serde_json::{json, Map, Value};

/// Most entries per list in one frame summary; `truncated` says when more exist.
pub(super) const MAX_LINKS: usize = 50;

/// One authored link and where it sits: its source node and that node's screen (the
/// page child that contains it).
pub(super) struct Edge<'a> {
    pub from: &'a Node,
    pub screen: &'a Node,
    pub link: Link,
}

/// Every link in project order: page, then screen, then preorder, then array order.
pub(super) fn edges<'a>(index: &TreeIndex<'a>) -> Vec<Edge<'a>> {
    let mut out = Vec::new();
    let mut screen = None;
    for (node, depth) in index.preorder(None) {
        if depth <= 1 {
            screen = Some(node);
        }
        let Some(screen) = screen.filter(|_| depth > 0) else {
            continue;
        };
        out.extend(node_links(&node.props).into_iter().map(|link| Edge {
            from: node,
            screen,
            link,
        }));
    }
    out
}

/// `{flowStart?, out?, in?, truncated?}` for one frame, or `None` when it has no outgoing
/// link in its subtree, no incoming link and no flow start.
pub(super) fn summary(index: &TreeIndex, frame: &str) -> Option<Value> {
    let root = index.get(frame)?;
    let out: Vec<Value> = index
        .preorder(Some(frame))
        .into_iter()
        .flat_map(|(node, _)| node_links(&node.props).into_iter().map(move |l| (node, l)))
        .map(|(node, link)| outgoing(index, node, link))
        .collect();
    let incoming: Vec<Value> = edges(index)
        .into_iter()
        .filter(|e| e.link.to.as_deref() == Some(frame))
        .map(|e| incoming(&e))
        .collect();
    let start = flow_start(&root.props);
    if out.is_empty() && incoming.is_empty() && start.is_none() {
        return None;
    }
    let mut summary = Map::new();
    if let Some(start) = start {
        summary.insert("flowStart".into(), json!(start));
    }
    let truncated = out.len() > MAX_LINKS || incoming.len() > MAX_LINKS;
    for (key, mut list) in [("out", out), ("in", incoming)] {
        list.truncate(MAX_LINKS);
        if !list.is_empty() {
            summary.insert(key.into(), Value::Array(list));
        }
    }
    if truncated {
        summary.insert("truncated".into(), json!(true));
    }
    Some(Value::Object(summary))
}

fn outgoing(index: &TreeIndex, node: &Node, link: Link) -> Value {
    let mut value = json!({"from":node.id,"on":link.on,"action":link.action});
    if let Some(to) = &link.to {
        value["to"] = json!(to);
        if let Some(name) = index.get(to).and_then(|t| t.props["name"].as_str()) {
            value["toName"] = json!(name);
        }
    }
    value
}

fn incoming(edge: &Edge) -> Value {
    let mut value = json!({"from":edge.from.id,"fromFrame":edge.screen.id});
    if let Some(name) = edge.screen.props["name"].as_str() {
        value["fromFrameName"] = json!(name);
    }
    value["on"] = json!(edge.link.on);
    value["action"] = json!(edge.link.action);
    value
}
