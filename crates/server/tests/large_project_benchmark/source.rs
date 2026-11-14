//! Private source design read from the wrapper's temporary directory, plus the fixed
//! edit/read targets. Nothing here is printed; reports only carry counts and timings.
use opencanvas_storage::{Node, NodeKind, Snapshot};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

pub struct Source {
    pub snapshot: Snapshot,
    /// Asset bytes by content hash, verified against the manifest hash.
    pub binaries: BTreeMap<String, Vec<u8>>,
}

pub fn load(dir: &Path) -> Source {
    let mut value: Value =
        serde_json::from_slice(&std::fs::read(dir.join("snapshot.json")).unwrap()).unwrap();
    // HTTP envelope fields are not part of the stored snapshot.
    let object = value.as_object_mut().unwrap();
    object.remove("truncated");
    object.remove("privacy_status");
    let snapshot: Snapshot = serde_json::from_value(value).unwrap();
    let mut binaries = BTreeMap::new();
    for asset in &snapshot.assets {
        let path = dir.join("assets").join(&asset.id);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            asset.content_hash,
            "asset bytes match the manifest"
        );
        binaries.insert(asset.content_hash.clone(), bytes);
    }
    Source { snapshot, binaries }
}

pub fn children(nodes: &[Node]) -> HashMap<Option<&str>, Vec<&Node>> {
    let mut map: HashMap<Option<&str>, Vec<&Node>> = HashMap::new();
    for node in nodes {
        map.entry(node.parent_id.as_deref()).or_default().push(node);
    }
    for list in map.values_mut() {
        list.sort_by(|a, b| a.sort_key.cmp(&b.sort_key).then(a.id.cmp(&b.id)));
    }
    map
}

/// Component masters and everything below them.
pub fn master_subtrees(nodes: &[Node]) -> HashSet<String> {
    let tree = children(nodes);
    let mut out = HashSet::new();
    let mut stack: Vec<&Node> = nodes
        .iter()
        .filter(|n| n.props.get("component").is_some())
        .collect();
    while let Some(node) = stack.pop() {
        if out.insert(node.id.clone()) {
            stack.extend(tree.get(&Some(node.id.as_str())).into_iter().flatten());
        }
    }
    out
}

pub struct Targets {
    pub page: String,
    pub frame: String,
    pub text: String,
}

/// First authored text outside every master, on the second or later page in page order.
pub fn targets(snapshot: &Snapshot) -> Targets {
    let tree = children(&snapshot.nodes);
    let masters = master_subtrees(&snapshot.nodes);
    for page in tree.get(&None).into_iter().flatten().skip(1) {
        let mut stack: Vec<(&Node, Option<&str>)> = vec![(page, None)];
        while let Some((node, root)) = stack.pop() {
            if masters.contains(&node.id) {
                continue;
            }
            if node.kind == NodeKind::Text {
                return Targets {
                    page: page.id.clone(),
                    frame: root.unwrap().to_string(),
                    text: node.id.clone(),
                };
            }
            let below = tree.get(&Some(node.id.as_str())).into_iter().flatten();
            for child in below.rev() {
                let root = root.or((node.kind == NodeKind::Page).then_some(child.id.as_str()));
                stack.push((child, root));
            }
        }
    }
    panic!("source has no editable text outside masters");
}
