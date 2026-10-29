//! One expansion boundary shared by compute and reads. Persistence stays authored.
use crate::{Node, NodeKind, Result, StorageError, MAX_NODES};
use serde_json::Value;
use std::collections::HashMap;

pub const OVERRIDE_KEYS: &[&str] = &["text", "fill", "visible", "styleId", "assetId", "name"];
pub const INSTANCE_KEYS: &[&str] = &[
    "componentId",
    "overrides",
    "name",
    "x",
    "y",
    "width",
    "height",
    "minWidth",
    "maxWidth",
    "minHeight",
    "maxHeight",
    "visible",
];

/// Expands references before resolving styles/tokens, preserving stable virtual IDs.
pub fn expand(nodes: &[Node]) -> Result<Vec<Node>> {
    if !nodes.iter().any(|n| n.kind == NodeKind::Instance) {
        return Ok(nodes.to_vec());
    }
    let by_id = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut children: HashMap<&str, Vec<&Node>> = HashMap::new();
    for node in nodes {
        if let Some(parent) = node.parent_id.as_deref() {
            children.entry(parent).or_default().push(node);
        }
    }
    let mut out = Vec::new();
    let expander = Expander { by_id, children };
    for node in nodes {
        expander.node(node.clone(), 0, &mut out)?;
    }
    Ok(out)
}

struct Expander<'a> {
    by_id: HashMap<&'a str, &'a Node>,
    children: HashMap<&'a str, Vec<&'a Node>>,
}
impl Expander<'_> {
    fn node(&self, node: Node, depth: usize, out: &mut Vec<Node>) -> Result<()> {
        if out.len() >= MAX_NODES {
            return Err(StorageError::new(
                "INSTANCE_LIMIT",
                "Expanded project exceeds 20000 nodes",
            ));
        }
        if node.kind != NodeKind::Instance {
            out.push(node);
            return Ok(());
        }
        if depth >= 8 {
            return Err(StorageError::new(
                "INSTANCE_DEPTH",
                "Component nesting exceeds 8",
            ));
        }
        let master = node.props["componentId"]
            .as_str()
            .and_then(|id| self.by_id.get(id))
            .filter(|n| n.kind == NodeKind::Frame && n.props.get("component").is_some())
            .ok_or_else(|| {
                StorageError::validation("props.componentId", "Expected a component frame")
            })?;
        let mut root = node.clone();
        root.kind = NodeKind::Frame;
        root.props = master.props.clone();
        root.props.as_object_mut().unwrap().remove("component");
        for (key, value) in node.props.as_object().unwrap() {
            if key != "componentId" && key != "overrides" {
                root.props[key] = value.clone();
            }
        }
        out.push(root);
        self.descendants(
            &master.id,
            &node.id,
            &node.id,
            &node.props["overrides"],
            depth + 1,
            out,
        )
    }

    fn descendants(
        &self,
        source: &str,
        parent: &str,
        prefix: &str,
        overrides: &Value,
        depth: usize,
        out: &mut Vec<Node>,
    ) -> Result<()> {
        for child in self.children.get(source).into_iter().flatten() {
            let mut virtual_node = (*child).clone();
            virtual_node.id = format!("{prefix}/{}", child.id);
            virtual_node.parent_id = Some(parent.into());
            if let Some(patch) = overrides.get(&child.id).and_then(Value::as_object) {
                virtual_node
                    .props
                    .as_object_mut()
                    .unwrap()
                    .extend(patch.clone());
            }
            let id = virtual_node.id.clone();
            let nested = virtual_node.kind == NodeKind::Instance;
            self.node(virtual_node, depth, out)?;
            if !nested {
                self.descendants(&child.id, &id, prefix, overrides, depth, out)?;
            }
        }
        Ok(())
    }
}
