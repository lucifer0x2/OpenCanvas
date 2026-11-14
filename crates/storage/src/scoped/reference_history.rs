//! Frozen pre-index reference algorithms, used only for differential tests.
use super::dependencies::page;
use crate::*;

pub(super) fn footprint(before: &Snapshot, after: &Candidate) -> Result<BTreeSet<String>> {
    let old = Candidate::from_snapshot(before);
    let mut keys = BTreeSet::new();
    if navigation(&old, after) {
        keys.insert("*".into());
    }
    for id in old
        .nodes
        .keys()
        .chain(after.nodes.keys())
        .collect::<BTreeSet<_>>()
    {
        if !same(old.nodes.get(id), after.nodes.get(id))? {
            keys.insert(format!("node:{id}"));
            for nodes in [&old.nodes, &after.nodes] {
                if let Some(page) = page(nodes, id) {
                    keys.insert(format!("page:{page}"));
                }
            }
        }
    }
    macro_rules! diff {
        ($field:ident, $kind:literal) => {
            for id in old
                .$field
                .keys()
                .chain(after.$field.keys())
                .collect::<BTreeSet<_>>()
            {
                if !same(old.$field.get(id), after.$field.get(id))? {
                    keys.insert(format!(concat!($kind, ":{}"), id));
                }
            }
        };
    }
    diff!(tokens, "token");
    diff!(styles, "style");
    diff!(assets, "asset");
    diff!(fonts, "font");
    diff!(notes, "note");
    for font in old.fonts.values().chain(after.fonts.values()) {
        if !same(old.fonts.get(&font.id), after.fonts.get(&font.id))? {
            keys.insert(format!("font-family:{}", font.family));
        }
    }
    Ok(keys)
}

fn same<T: serde::Serialize>(before: Option<&T>, after: Option<&T>) -> Result<bool> {
    Ok(serde_json::to_vec(&before)? == serde_json::to_vec(&after)?)
}

fn navigation(before: &Candidate, after: &Candidate) -> bool {
    let pages = |candidate: &Candidate| {
        candidate
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::Page)
            .cloned()
            .collect::<Vec<_>>()
    };
    if pages(before) != pages(after) {
        return true;
    }
    for id in before
        .nodes
        .keys()
        .chain(after.nodes.keys())
        .collect::<BTreeSet<_>>()
    {
        if before.nodes.get(id).and_then(|n| n.props.get("flowStart"))
            != after.nodes.get(id).and_then(|n| n.props.get("flowStart"))
        {
            return true;
        }
    }
    for page in before.nodes.values().filter(|n| n.kind == NodeKind::Page) {
        let old = roots(before, &page.id);
        let new = roots(after, &page.id);
        if !new.starts_with(&old) || (old.is_empty() && !new.is_empty()) {
            return true;
        }
    }
    false
}

fn roots(candidate: &Candidate, page: &str) -> Vec<String> {
    let mut nodes: Vec<_> = candidate
        .nodes
        .values()
        .filter(|n| n.kind == NodeKind::Frame && n.parent_id.as_deref() == Some(page))
        .map(|n| (n.sort_key, n.id.clone()))
        .collect();
    nodes.sort();
    nodes.into_iter().map(|(_, id)| id).collect()
}
