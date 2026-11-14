//! Independent closure reference: which full-expansion nodes a scope must return.
use opencanvas_scene_input::{instances, read_scope::ReadScope, Snapshot};
use std::collections::BTreeSet;

fn descendants(snapshot: &Snapshot, id: &str, out: &mut BTreeSet<String>) {
    let mut seen = BTreeSet::new();
    let mut stack = vec![id.to_owned()];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) || !snapshot.nodes.iter().any(|n| n.id == id) {
            continue;
        }
        let children = snapshot
            .nodes
            .iter()
            .filter(|n| n.parent_id == Some(id.clone()));
        stack.extend(children.map(|n| n.id.clone()));
        out.insert(id);
    }
}

fn ancestors(snapshot: &Snapshot, id: &str, out: &mut BTreeSet<String>) {
    let mut current = snapshot.nodes.iter().find(|n| n.id == id);
    while let Some(node) = current {
        out.insert(node.id.clone());
        let parent = node.parent_id.as_deref();
        current = snapshot
            .nodes
            .iter()
            .find(|n| Some(n.id.as_str()) == parent);
    }
}

/// Full-expansion IDs (in full order) whose outer authored node lies in the target's
/// subtree/ancestry, plus transitive master trees when definitions are requested.
pub fn reference_ids(snapshot: &Snapshot, scope: ReadScope<'_>, definitions: bool) -> Vec<String> {
    let id = match scope {
        ReadScope::Page(id) | ReadScope::Target(id) => id,
    };
    let authored = id.split('/').next().unwrap();
    let mut selected = BTreeSet::new();
    descendants(snapshot, authored, &mut selected);
    ancestors(snapshot, authored, &mut selected);
    let mut done = BTreeSet::new();
    while definitions {
        let masters: Vec<_> = snapshot
            .nodes
            .iter()
            .filter(|n| selected.contains(&n.id))
            .filter_map(|n| n.props["componentId"].as_str())
            .filter(|m| !done.contains(*m))
            .map(str::to_owned)
            .collect();
        if masters.is_empty() {
            break;
        }
        for master in masters {
            descendants(snapshot, &master, &mut selected);
            ancestors(snapshot, &master, &mut selected);
            done.insert(master);
        }
    }
    instances::expand(&snapshot.nodes)
        .unwrap()
        .into_iter()
        .filter(|n| selected.contains(n.id.split('/').next().unwrap()))
        .map(|n| n.id)
        .collect()
}
