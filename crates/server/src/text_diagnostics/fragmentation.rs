//! Conservative multi-signal heuristic for likely captured paragraph characters.
use super::intent;
use opencanvas_storage::{Node, NodeKind, Snapshot};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) fn authored(snapshot: &Snapshot, nodes: &[Node], frame: &str) -> Vec<Value> {
    let intent = intent::Intent::new(snapshot);
    let by_id: HashMap<_, _> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut children: BTreeMap<&str, Vec<&Node>> = BTreeMap::new();
    let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in nodes {
        if let Some(parent) = node.parent_id.as_deref() {
            edges.entry(parent).or_default().push(&node.id);
        }
    }
    let mut scoped = HashSet::new();
    let mut pending = vec![frame];
    while let Some(id) = pending.pop() {
        if scoped.insert(id) {
            pending.extend(edges.get(id).into_iter().flatten().copied());
        }
    }
    for node in nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Text && scoped.contains(n.id.as_str()))
    {
        let mut current = Some(node);
        let mut excluded = false;
        while let Some(ancestor) = current {
            excluded |= ancestor.props["visible"] == false || intent.art(ancestor);
            current = ancestor
                .parent_id
                .as_deref()
                .and_then(|id| by_id.get(id).copied());
        }
        if !excluded {
            if let Some(parent) = node.parent_id.as_deref() {
                children.entry(parent).or_default().push(node);
            }
        }
    }
    let mut issues = Vec::new();
    for (parent, siblings) in children {
        let Some(container) = by_id.get(parent) else {
            continue;
        };
        if matches!(
            container.props["layout"]["mode"].as_str(),
            Some("row" | "column")
        ) {
            continue;
        }
        let mut groups: BTreeMap<String, Vec<&Node>> = BTreeMap::new();
        for node in siblings {
            let text = node.props["text"].as_str().unwrap_or("");
            if text.chars().count() > 3 || text.chars().filter(|c| c.is_alphabetic()).count() != 1 {
                continue;
            }
            let p = &node.props;
            if p["fontSize"].as_f64().is_none() || p["lineHeight"].as_f64().is_none() {
                continue;
            }
            let key = json!([
                p["fontId"],
                p["fontFamily"],
                p["fontSize"],
                p["lineHeight"],
                p["fill"],
                p["fontWeight"],
                p["fontStyle"],
                p["letterSpacing"],
                p["textTransform"],
                p["textAlign"],
                p["textDecoration"],
                p["y"].as_f64().unwrap_or(0.).round()
            ])
            .to_string();
            groups.entry(key).or_default().push(node);
        }
        for mut group in groups.into_values() {
            if group.len() < 8 {
                continue;
            }
            group.sort_by(|a, b| {
                number(a, "x")
                    .total_cmp(&number(b, "x"))
                    .then(a.id.cmp(&b.id))
            });
            let size = number(group[0], "fontSize").max(1.);
            // Dense adjacent placements plus a shared baseline/style and many letter fragments.
            let close = group
                .windows(2)
                .filter(|w| {
                    let gap = number(w[1], "x") - number(w[0], "x");
                    gap > 0. && gap <= size * 1.5
                })
                .count();
            if close * 5 < (group.len() - 1) * 4 {
                continue;
            }
            issues.push(json!({"code":"TEXT_FRAGMENTATION_SUSPECTED","severity":"warning",
                "frameId":frame,"parentId":parent,"scope":"full-effective-expanded-frame-before-pagination",
                "fragmentCount":group.len(),"nodeIds":group.iter().take(6).map(|n| &n.id).collect::<Vec<_>>(),
                "evidence":["many single-letter siblings","same baseline and typography","dense free placements"],
                "message":"Possible fragmented semantic copy. Read authored source and notes; intentional lettering is valid. Do not merge or reconstruct spaces/order automatically."}));
        }
    }
    let total = issues.len();
    issues.truncate(8);
    if total > 8 {
        issues.push(json!({"code":"TEXT_FRAGMENTATION_SUMMARY","scope":"full-effective-expanded-frame-before-pagination","omittedGroups":total-8}));
    }
    issues
}
fn number(node: &Node, key: &str) -> f64 {
    node.props[key].as_f64().unwrap_or(0.)
}
