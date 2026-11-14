//! Frozen pre-index reference algorithms, used only for differential tests.
use super::dependencies::page;
use crate::*;

pub(super) fn footprint(before: &Snapshot, after: &Candidate) -> Result<BTreeSet<String>> {
    let old = Candidate::from_snapshot(before);
    let mut keys = BTreeSet::new();
    let global = navigation(&old, after);
    if global {
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
            let (a, b) = (old.nodes.get(id), after.nodes.get(id));
            if let (false, Some(a), Some(b)) = (global, a, b) {
                let mut b = b.clone();
                b.sort_key = a.sort_key;
                if a.kind == NodeKind::Page && same(Some(a), Some(&b))? {
                    continue;
                }
            }
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
    for id in old.notes.keys().chain(after.notes.keys()) {
        if !same(old.notes.get(id), after.notes.get(id))? {
            for (notes, nodes) in [(&old.notes, &old.nodes), (&after.notes, &after.nodes)] {
                if let Some(page) = notes.get(id).and_then(|n| page(nodes, &n.node_id)) {
                    keys.insert(format!("page:{page}"));
                }
            }
        }
    }
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
        let mut pages: Vec<_> = candidate
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::Page)
            .map(|n| (n.sort_key, n.id.clone()))
            .collect();
        pages.sort();
        pages.into_iter().map(|(_, id)| id).collect::<Vec<_>>()
    };
    let (old_pages, new_pages) = (pages(before), pages(after));
    // Only appending pages to a project that already had pages is local.
    if old_pages.is_empty() && !new_pages.is_empty() || !new_pages.starts_with(&old_pages) {
        return true;
    }
    for id in &old_pages {
        let (a, b) = (&before.nodes[id], &after.nodes[id]);
        if (&a.kind, &a.parent_id, &a.props) != (&b.kind, &b.parent_id, &b.props) {
            return true;
        }
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
    for page in &old_pages {
        if !roots(after, page).starts_with(&roots(before, page)) {
            return true;
        }
    }
    let first = |candidate: &Candidate, pages: &[String]| {
        pages
            .iter()
            .find_map(|page| roots(candidate, page).into_iter().next())
    };
    first(before, &old_pages) != first(after, &new_pages)
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
