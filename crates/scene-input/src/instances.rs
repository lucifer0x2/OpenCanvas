//! One expansion boundary shared by compute and reads. Persistence stays authored.
use crate::{Node, NodeKind, Result, StorageError, MAX_NODES};
use serde_json::Value;
use std::collections::HashMap;

const MAX_TREE_DEPTH: usize = 128;

/// Stored rows are trusted on scoped reads; malformed props must fail, never panic.
fn not_object() -> StorageError {
    StorageError::validation("props", "Properties must be an object")
}

pub const OVERRIDE_KEYS: &[&str] = &[
    "text", "fill", "visible", "styleId", "assetId", "name", "stroke", "shadow", "states",
    "opacity", "radius",
];
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
    "theme",
    "fill",
    "stroke",
    "shadow",
    "radius",
    "opacity",
];

/// Expands references before resolving styles/tokens, preserving stable virtual IDs.
pub fn expand(nodes: &[Node]) -> Result<Vec<Node>> {
    if !nodes.iter().any(|n| n.kind == NodeKind::Instance) {
        return Ok(nodes.to_vec());
    }
    let nodes: Vec<_> = nodes.iter().collect();
    expand_selected(&nodes, &nodes)
}

/// Authored lookups for expansion: a bounded closure or a revision-bound index.
/// Children must be returned in snapshot source order.
pub(crate) trait Lookup {
    fn node(&self, id: &str) -> Option<&Node>;
    fn children(&self, id: &str) -> Children<'_>;
}

/// Allocation-free child iteration over either lookup representation.
pub(crate) enum Children<'s> {
    Nodes(std::slice::Iter<'s, &'s Node>),
    Positions(std::slice::Iter<'s, usize>, &'s [Node]),
}

impl<'s> Iterator for Children<'s> {
    type Item = &'s Node;
    fn next(&mut self) -> Option<&'s Node> {
        match self {
            Self::Nodes(nodes) => nodes.next().copied(),
            Self::Positions(positions, nodes) => positions.next().map(|&i| &nodes[i]),
        }
    }
}

struct Closure<'a> {
    by_id: HashMap<&'a str, &'a Node>,
    children: HashMap<&'a str, Vec<&'a Node>>,
}

impl Lookup for Closure<'_> {
    fn node(&self, id: &str) -> Option<&Node> {
        self.by_id.get(id).copied()
    }
    fn children(&self, id: &str) -> Children<'_> {
        Children::Nodes(self.children.get(id).map_or([].iter(), |c| c.iter()))
    }
}

/// Expand selected authored nodes using an already bounded dependency closure.
/// Both slices retain source order; masters are lookup-only unless selected.
fn expand_selected(nodes: &[&Node], selected: &[&Node]) -> Result<Vec<Node>> {
    let by_id = nodes.iter().map(|&n| (n.id.as_str(), n)).collect();
    let mut children: HashMap<&str, Vec<&Node>> = HashMap::new();
    for &node in nodes {
        if let Some(parent) = node.parent_id.as_deref() {
            children.entry(parent).or_default().push(node);
        }
    }
    expand_with(&Closure { by_id, children }, selected)
}

/// Expand `selected` in order. Any lookup that resolves every reachable master
/// and its complete subtree produces the same output as the full expansion.
pub(crate) fn expand_with(lookup: &impl Lookup, selected: &[&Node]) -> Result<Vec<Node>> {
    let mut out = Vec::new();
    let expander = Expander { lookup };
    for &node in selected {
        expander.node(node.clone(), 0, &mut out)?;
    }
    Ok(out)
}

struct Expander<'l, L> {
    lookup: &'l L,
}
impl<L: Lookup> Expander<'_, L> {
    fn node(&self, node: Node, depth: usize, out: &mut Vec<Node>) -> Result<()> {
        if out.len() >= MAX_NODES {
            return Err(StorageError::new(
                "INSTANCE_LIMIT",
                &format!("Expanded project exceeds {MAX_NODES} nodes"),
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
            .and_then(|id| self.lookup.node(id))
            .filter(|n| n.kind == NodeKind::Frame && n.props.get("component").is_some())
            .ok_or_else(|| {
                StorageError::validation("props.componentId", "Expected a component frame")
            })?;
        let mut root = node.clone();
        root.kind = NodeKind::Frame;
        root.props = master.props.clone();
        root.props
            .as_object_mut()
            .ok_or_else(not_object)?
            .remove("component");
        let mut patch = node.props.as_object().ok_or_else(not_object)?.clone();
        patch.remove("componentId");
        patch.remove("overrides");
        crate::instance_paint::merge_props(&mut root.props, &patch);
        out.push(root);
        self.descendants(
            &master.id,
            &node.id,
            &node.id,
            &node.props["overrides"],
            (depth + 1, 1),
            out,
        )
    }

    fn descendants(
        &self,
        source: &str,
        parent: &str,
        prefix: &str,
        overrides: &Value,
        (depth, level): (usize, usize),
        out: &mut Vec<Node>,
    ) -> Result<()> {
        // A validated master subtree is at most 128 levels deep; deeper walks
        // can only come from a cycle or corrupt stored rows.
        if level > MAX_TREE_DEPTH {
            return Err(StorageError::validation(
                "parentId",
                "Tree exceeds maximum depth 128",
            ));
        }
        for child in self.lookup.children(source) {
            let mut virtual_node = child.clone();
            virtual_node.id = format!("{prefix}/{}", child.id);
            virtual_node.parent_id = Some(parent.into());
            if let Some(patch) = overrides.get(&child.id).and_then(Value::as_object) {
                virtual_node.props.as_object().ok_or_else(not_object)?;
                crate::instance_paint::merge_props(&mut virtual_node.props, patch);
            }
            let id = virtual_node.id.clone();
            let nested = virtual_node.kind == NodeKind::Instance;
            self.node(virtual_node, depth, out)?;
            if !nested {
                self.descendants(&child.id, &id, prefix, overrides, (depth, level + 1), out)?;
            }
        }
        Ok(())
    }
}
