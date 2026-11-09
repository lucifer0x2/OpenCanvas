use crate::*;

pub(super) fn page(nodes: &BTreeMap<String, Node>, id: &str) -> Option<String> {
    let mut node = nodes.get(id)?;
    for _ in 0..=nodes.len() {
        if node.kind == NodeKind::Page {
            return Some(node.id.clone());
        }
        node = nodes.get(node.parent_id.as_ref()?)?;
    }
    None
}

pub(super) fn closure(snapshot: &Snapshot, pages: &[String]) -> Result<BTreeSet<String>> {
    if pages.is_empty()
        || pages.len() > 100
        || pages.iter().collect::<BTreeSet<_>>().len() != pages.len()
    {
        return Err(StorageError::validation(
            "pageIds",
            "Supply 1–100 distinct existing page IDs",
        ));
    }
    let candidate = Candidate::from_snapshot(snapshot);
    let expanded = crate::instances::expand(&snapshot.nodes)?;
    let expanded_map = expanded
        .iter()
        .cloned()
        .map(|n| (n.id.clone(), n))
        .collect();
    let mut keys = BTreeSet::new();
    let mut queue = Vec::new();
    for id in pages {
        if candidate
            .nodes
            .get(id)
            .is_none_or(|n| n.kind != NodeKind::Page)
        {
            return Err(StorageError::validation(
                "pageIds",
                "Guarded page does not exist",
            ));
        }
        queue.push(format!("page:{id}"));
    }
    while let Some(key) = queue.pop() {
        if !keys.insert(key.clone()) {
            continue;
        }
        let (kind, id) = key.split_once(':').unwrap();
        match kind {
            "page" => {
                let nodes: Vec<_> = expanded
                    .iter()
                    .filter(|n| page(&expanded_map, &n.id).as_deref() == Some(id))
                    .collect();
                // Authored refs retain dependency edges hidden by an override.
                for node in candidate
                    .nodes
                    .values()
                    .filter(|n| page(&candidate.nodes, &n.id).as_deref() == Some(id))
                {
                    scan(&node.props, &mut queue);
                }
                for node in &nodes {
                    scan(&node.props, &mut queue);
                }
                // Font face selection can depend on family siblings not explicitly
                // named in authored properties, under every supported theme.
                for font in crate::referenced_font_ids(snapshot, &nodes)? {
                    queue.push(format!("font:{font}"));
                }
            }
            "token" => {
                if let Some(t) = candidate.tokens.get(id) {
                    scan(&t.value, &mut queue);
                }
            }
            "style" => {
                if let Some(s) = candidate.styles.get(id) {
                    scan(&s.value, &mut queue);
                }
            }
            "component" | "link" => {
                if let Some(page) = page(&candidate.nodes, id) {
                    queue.push(format!("page:{page}"));
                }
            }
            "font" => {
                if let Some(font) = candidate.fonts.get(id) {
                    queue.push(format!("font-family:{}", font.family));
                    if let Some(asset) = &font.asset_id {
                        queue.push(format!("asset:{asset}"));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(keys)
}

fn scan(value: &Value, queue: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if let Some(id) = value.as_str() {
                    let kind = match key.as_str() {
                        "tokenId" | "aliasId" => Some("token"),
                        "styleId" => Some("style"),
                        "fontId" => Some("font"),
                        "assetId" => Some("asset"),
                        "componentId" => Some("component"),
                        "to" => Some("link"),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        queue.push(format!("{kind}:{id}"));
                    }
                }
                scan(value, queue);
            }
        }
        Value::Array(values) => {
            for value in values {
                scan(value, queue);
            }
        }
        _ => {}
    }
}
