//! Revision-local dependency adjacency; closure visits pages and references once.
use super::index::PageIndex;
use crate::*;
use validate::FontReferences;

pub(crate) struct DependencyIndex {
    pub tree: Arc<PageIndex>,
    references: BTreeMap<String, BTreeSet<String>>,
    page_fonts: BTreeMap<String, BTreeSet<String>>,
}

struct Authored<'a> {
    nodes: Vec<&'a Node>,
    references: Option<&'a BTreeMap<String, BTreeSet<String>>>,
}

impl DependencyIndex {
    #[cfg(test)]
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self> {
        Self::with_snapshot_tree(snapshot, Arc::new(PageIndex::new(&snapshot.nodes)), None)
    }

    pub fn with_snapshot_tree(
        snapshot: &Snapshot,
        tree: Arc<PageIndex>,
        references: Option<&BTreeMap<String, BTreeSet<String>>>,
    ) -> Result<Self> {
        let expanded = bench_phase::time("scoped.expand.before", || {
            crate::instances::expand(&snapshot.nodes)
        })?
        .into_iter()
        .map(|n| (n.id.clone(), n))
        .collect();
        let fonts = FontReferences::from_snapshot(snapshot, &expanded)?;
        Self::build(
            Authored {
                nodes: snapshot.nodes.iter().collect(),
                references,
            },
            tree,
            &expanded,
            &snapshot.tokens,
            &snapshot.styles,
            &snapshot.fonts,
            &fonts,
        )
    }

    pub fn from_candidate(
        candidate: &Candidate,
        expanded: Option<BTreeMap<String, Node>>,
        tree: Arc<PageIndex>,
        references: Option<&BTreeMap<String, BTreeSet<String>>>,
    ) -> Result<Self> {
        // Full-validation fallback and instance-free incremental validation may
        // supply no expansion. Absence never means an empty scene.
        bench_phase::count("scoped.expansion_reused", usize::from(expanded.is_some()));
        let expanded = match expanded {
            Some(nodes) => nodes,
            None => bench_phase::time("scoped.expand.fallback", || {
                crate::instances::expand(&candidate.nodes.values().cloned().collect::<Vec<_>>())
            })?
            .into_iter()
            .map(|n| (n.id.clone(), n))
            .collect(),
        };
        let fonts = FontReferences::from_candidate(candidate, &expanded)?;
        Self::build(
            Authored {
                nodes: candidate.nodes.values().collect(),
                references,
            },
            tree,
            &expanded,
            candidate.tokens.values(),
            candidate.styles.values(),
            candidate.fonts.values(),
            &fonts,
        )
    }

    fn build<'a>(
        authored: Authored<'_>,
        tree: Arc<PageIndex>,
        expanded: &BTreeMap<String, Node>,
        tokens: impl IntoIterator<Item = &'a Token>,
        styles: impl IntoIterator<Item = &'a Style>,
        fonts: impl IntoIterator<Item = &'a Font>,
        font_refs: &FontReferences<'_>,
    ) -> Result<Self> {
        let expanded_pages = PageIndex::new(expanded.values());
        let mut references = BTreeMap::new();
        if let Some(usage) = authored.references {
            for (key, users) in usage {
                for user in users {
                    if let Some(page) = tree.page(user) {
                        references
                            .entry(format!("page:{page}"))
                            .or_insert_with(BTreeSet::new)
                            .insert(key.clone());
                    }
                }
            }
        }
        // Both representations matter: overrides can hide authored references.
        for (node, pages) in authored
            .nodes
            .into_iter()
            .filter(|_| authored.references.is_none())
            .map(|n| (n, tree.as_ref()))
            .chain(expanded.values().map(|n| (n, &expanded_pages)))
        {
            if let Some(page) = pages.page(&node.id) {
                scan(
                    &node.props,
                    references.entry(format!("page:{page}")).or_default(),
                );
            }
        }
        for token in tokens {
            scan(
                &token.value,
                references.entry(format!("token:{}", token.id)).or_default(),
            );
        }
        for style in styles {
            scan(
                &style.value,
                references.entry(format!("style:{}", style.id)).or_default(),
            );
        }
        for font in fonts {
            let refs = references.entry(format!("font:{}", font.id)).or_default();
            refs.insert(format!("font-family:{}", font.family));
            if let Some(asset) = &font.asset_id {
                refs.insert(format!("asset:{asset}"));
            }
        }
        let mut page_fonts = BTreeMap::new();
        for (page, members) in &expanded_pages.page_nodes {
            let nodes: Vec<_> = members.iter().filter_map(|id| expanded.get(id)).collect();
            page_fonts.insert(page.clone(), font_refs.for_nodes(&nodes)?);
        }
        Ok(Self {
            tree,
            references,
            page_fonts,
        })
    }

    pub fn closure(&self, pages: &[String]) -> Result<BTreeSet<String>> {
        self.tree.validate_pages(pages)?;
        let mut keys = BTreeSet::new();
        let mut queue: Vec<_> = pages.iter().map(|p| format!("page:{p}")).collect();
        while let Some(key) = queue.pop() {
            if !keys.insert(key.clone()) {
                continue;
            }
            if let Some(refs) = self.references.get(&key) {
                queue.extend(refs.iter().cloned());
            }
            let (kind, id) = key.split_once(':').unwrap();
            match kind {
                "page" => {
                    queue.extend(
                        self.page_fonts
                            .get(id)
                            .into_iter()
                            .flatten()
                            .map(|id| format!("font:{id}")),
                    );
                }
                "component" | "link" => {
                    if let Some(page) = self.tree.page(id) {
                        queue.push(format!("page:{page}"));
                    }
                }
                _ => {}
            }
        }
        bench_phase::count("scoped.closure_keys", keys.len());
        Ok(keys)
    }

    pub(crate) fn estimated_bytes(&self) -> usize {
        self.references
            .iter()
            .chain(&self.page_fonts)
            .map(|(key, refs)| 96 + key.len() + refs.iter().map(|id| 64 + id.len()).sum::<usize>())
            .sum()
    }
}

fn scan(value: &Value, keys: &mut BTreeSet<String>) {
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
                        keys.insert(format!("{kind}:{id}"));
                    }
                }
                scan(value, keys);
            }
        }
        Value::Array(values) => {
            for value in values {
                scan(value, keys);
            }
        }
        _ => {}
    }
}
