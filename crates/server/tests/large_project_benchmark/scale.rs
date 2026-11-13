//! Deterministic page cloning. Copy k (1-based, the source is copy 0) suffixes every
//! cloned ID with `-xKK`; references to cloned nodes follow, other references stay.
//! Tokens, styles, fonts and assets are shared, as in one real large project.
use super::source::master_subtrees;
use opencanvas_storage::{Node, NodeKind, Note, Snapshot};
use serde_json::{Map, Value};
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    /// Every copy clones its masters; instances point at their own copy's masters.
    Own,
    /// Copies omit master subtrees; their instances keep pointing at the source masters.
    Shared,
}

impl Variant {
    pub fn parse(text: &str) -> Self {
        match text {
            "own" => Self::Own,
            "shared" => Self::Shared,
            other => panic!("unknown variant {other}"),
        }
    }
}

/// Keys whose string values are user text, never node references.
const TEXT_KEYS: &[&str] = &["name", "text"];

pub fn build(source: &Snapshot, copies: usize, variant: Variant) -> Snapshot {
    assert!(copies >= 1);
    let mut out = source.clone();
    let skipped = match variant {
        Variant::Own => HashSet::new(),
        Variant::Shared => master_subtrees(&source.nodes),
    };
    let cloned: HashSet<&str> = source
        .nodes
        .iter()
        .filter(|n| !skipped.contains(&n.id))
        .map(|n| n.id.as_str())
        .collect();
    let pages = source
        .nodes
        .iter()
        .filter(|n| n.parent_id.is_none())
        .count() as i64;
    for copy in 1..copies {
        let suffix = format!("-x{copy:02}");
        let map = |id: &str| -> String {
            if cloned.contains(id) {
                format!("{id}{suffix}")
            } else {
                id.to_string()
            }
        };
        for node in source
            .nodes
            .iter()
            .filter(|n| cloned.contains(n.id.as_str()))
        {
            let mut props = node.props.clone();
            rewrite(&mut props, &map, None);
            if node.kind == NodeKind::Page {
                let name = props["name"].as_str().unwrap_or("Page").to_string();
                props["name"] = Value::String(format!("{name} copy {copy}"));
            }
            out.nodes.push(Node {
                id: map(&node.id),
                parent_id: node.parent_id.as_deref().map(map),
                sort_key: if node.parent_id.is_none() {
                    node.sort_key + pages * copy as i64
                } else {
                    node.sort_key
                },
                kind: node.kind.clone(),
                props,
            });
        }
        for note in source
            .notes
            .iter()
            .filter(|n| cloned.contains(n.node_id.as_str()))
        {
            out.notes.push(Note {
                id: format!("{}{suffix}", note.id),
                node_id: map(&note.node_id),
                ..note.clone()
            });
        }
    }
    out
}

fn rewrite(value: &mut Value, map: &impl Fn(&str) -> String, key: Option<&str>) {
    match value {
        Value::String(text) if !key.is_some_and(|k| TEXT_KEYS.contains(&k)) => {
            *text = map(text);
        }
        Value::Array(items) => items.iter_mut().for_each(|v| rewrite(v, map, key)),
        Value::Object(object) => {
            let overrides = key == Some("overrides");
            let entries = std::mem::take(object);
            let mut next = Map::new();
            for (k, mut v) in entries {
                rewrite(&mut v, map, Some(&k));
                next.insert(if overrides { map(&k) } else { k }, v);
            }
            *object = next;
        }
        _ => {}
    }
}
