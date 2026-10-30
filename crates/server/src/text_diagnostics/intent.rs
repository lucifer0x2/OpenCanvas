//! Explicit note markers carry intent without adding unsupported authored properties.
use opencanvas_storage::{Node, Snapshot};
use std::collections::{HashMap, HashSet};

pub(super) struct Intent<'a> {
    notes: HashMap<&'a str, (bool, bool)>,
    masters: HashMap<&'a str, &'a str>,
}
impl<'a> Intent<'a> {
    pub fn new(snapshot: &'a Snapshot) -> Self {
        let mut notes: HashMap<&str, (bool, bool)> = HashMap::new();
        for note in &snapshot.notes {
            let flags = notes.entry(&note.node_id).or_default();
            let text = note.text.to_ascii_lowercase();
            flags.0 |= text.contains("intentional lettering:");
            flags.1 |= text.contains("intentional overflow:");
        }
        let masters = snapshot
            .nodes
            .iter()
            .filter_map(|n| Some((n.id.as_str(), n.props["componentId"].as_str()?)))
            .collect();
        Self { notes, masters }
    }
    pub fn noted(&self, node: &Node, overflow: bool) -> bool {
        let source = node.id.rsplit('/').next().unwrap_or(&node.id);
        [
            Some(node.id.as_str()),
            Some(source),
            self.masters.get(source).copied(),
        ]
        .into_iter()
        .flatten()
        .filter_map(|id| self.notes.get(id))
        .any(|flags| if overflow { flags.1 } else { flags.0 })
    }
    pub fn art(&self, node: &Node) -> bool {
        self.noted(node, false)
            || [
                "wordmark",
                "monogram",
                "letter art",
                "letter motion",
                "logo",
            ]
            .iter()
            .any(|term| {
                node.props["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains(term)
            })
            || node.props["animations"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
    }
}

pub(crate) fn overflow_ids(snapshot: &Snapshot, nodes: &[Node]) -> Vec<String> {
    let intent = Intent::new(snapshot);
    let by_id: HashMap<_, _> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut marked = HashSet::new();
    for node in nodes {
        let mut current = Some(node);
        while let Some(ancestor) = current {
            if intent.noted(ancestor, true) {
                marked.insert(node.id.clone());
                break;
            }
            current = ancestor
                .parent_id
                .as_deref()
                .and_then(|id| by_id.get(id).copied());
        }
    }
    let mut marked: Vec<_> = marked.into_iter().collect();
    marked.sort();
    marked
}
