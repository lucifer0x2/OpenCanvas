//! One-pass parent/child index over a snapshot, shared by every tree-shaped read.
//! Sibling order is storage order followed by a stable sort on sortKey: the order reads have
//! always returned, now built in O(n log n) instead of rescanning all nodes per node.
use opencanvas_storage::Node;
use std::collections::HashMap;

pub(super) struct TreeIndex<'a> {
    roots: Vec<&'a Node>,
    children: HashMap<&'a str, Vec<&'a Node>>,
    by_id: HashMap<&'a str, &'a Node>,
}

impl<'a> TreeIndex<'a> {
    pub fn new(nodes: &'a [Node]) -> Self {
        let mut roots = Vec::new();
        let mut children: HashMap<&str, Vec<&Node>> = HashMap::new();
        let mut by_id = HashMap::with_capacity(nodes.len());
        for node in nodes {
            match node.parent_id.as_deref() {
                Some(parent) => children.entry(parent).or_default().push(node),
                None => roots.push(node),
            }
            by_id.insert(node.id.as_str(), node);
        }
        roots.sort_by_key(|n| n.sort_key);
        for siblings in children.values_mut() {
            siblings.sort_by_key(|n| n.sort_key);
        }
        Self {
            roots,
            children,
            by_id,
        }
    }

    pub fn get(&self, id: &str) -> Option<&'a Node> {
        self.by_id.get(id).copied()
    }

    pub fn child_count(&self, id: &str) -> usize {
        self.children.get(id).map_or(0, Vec::len)
    }

    /// Preorder `(node, depth)` from one root, or from every top-level node; a root has depth 0.
    pub fn preorder(&self, root: Option<&str>) -> Vec<(&'a Node, usize)> {
        let starts = match root {
            Some(id) => self.get(id).into_iter().collect(),
            None => self.roots.clone(),
        };
        let mut out = Vec::new();
        let mut stack: Vec<_> = starts.into_iter().rev().map(|n| (n, 0)).collect();
        while let Some((node, depth)) = stack.pop() {
            out.push((node, depth));
            let children = self
                .children
                .get(node.id.as_str())
                .map_or(&[][..], Vec::as_slice);
            stack.extend(children.iter().rev().map(|child| (*child, depth + 1)));
        }
        out
    }

    /// Ancestors of `id`, root first and ending at its parent. With `scope`, the chain starts at
    /// that scope root; the scope root itself has no ancestors inside its scope.
    pub fn ancestors(&self, id: &str, scope: Option<&str>) -> Vec<&'a Node> {
        let mut chain = Vec::new();
        let mut current = self.get(id).filter(|_| scope != Some(id));
        while let Some(parent) = current
            .and_then(|n| n.parent_id.as_deref())
            .and_then(|p| self.get(p))
        {
            if chain.len() >= self.by_id.len() {
                break; // Storage rejects cycles; never loop on a corrupt snapshot.
            }
            chain.push(parent);
            current = Some(parent).filter(|p| scope != Some(p.id.as_str()));
        }
        chain.reverse();
        chain
    }
}
