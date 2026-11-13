//! Revision-local dependency adjacency; closure visits pages and references once.
use super::index::PageIndex;
use crate::*;
use validate::FontReferences;

pub(crate) struct DependencyIndex {
    pub tree: Arc<PageIndex>,
    pub expanded: BTreeMap<String, Node>,
    expanded_pages: PageIndex,
    references: BTreeMap<String, BTreeSet<String>>,
}

impl DependencyIndex {
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self> {
        Self::with_snapshot_tree(snapshot, Arc::new(PageIndex::new(&snapshot.nodes)))
    }

    pub fn with_snapshot_tree(snapshot: &Snapshot, tree: Arc<PageIndex>) -> Result<Self> {
        let expanded = bench_phase::time("scoped.expand.before", || {
            crate::instances::expand(&snapshot.nodes)
        })?
        .into_iter()
        .map(|n| (n.id.clone(), n))
        .collect();
        Ok(Self::build(
            &snapshot.nodes,
            tree,
            expanded,
            &snapshot.tokens,
            &snapshot.styles,
            &snapshot.fonts,
        ))
    }

    pub fn from_candidate(
        candidate: &Candidate,
        expanded: Option<BTreeMap<String, Node>>,
        tree: Arc<PageIndex>,
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
        Ok(Self::build(
            candidate.nodes.values(),
            tree,
            expanded,
            candidate.tokens.values(),
            candidate.styles.values(),
            candidate.fonts.values(),
        ))
    }

    fn build<'a>(
        nodes: impl IntoIterator<Item = &'a Node>,
        tree: Arc<PageIndex>,
        expanded: BTreeMap<String, Node>,
        tokens: impl IntoIterator<Item = &'a Token>,
        styles: impl IntoIterator<Item = &'a Style>,
        fonts: impl IntoIterator<Item = &'a Font>,
    ) -> Self {
        let nodes: Vec<_> = nodes.into_iter().collect();
        let expanded_pages = PageIndex::new(expanded.values());
        let mut references = BTreeMap::new();
        // Both representations matter: overrides can hide authored references.
        for (node, pages) in nodes
            .into_iter()
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
        Self {
            tree,
            expanded,
            expanded_pages,
            references,
        }
    }

    pub fn closure(
        &self,
        pages: &[String],
        fonts: &FontReferences<'_>,
    ) -> Result<BTreeSet<String>> {
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
                    let nodes: Vec<_> = self
                        .expanded_pages
                        .page_nodes
                        .get(id)
                        .into_iter()
                        .flatten()
                        .filter_map(|id| self.expanded.get(id))
                        .collect();
                    bench_phase::count("scoped.closure_nodes", nodes.len());
                    queue.extend(
                        fonts
                            .for_nodes(&nodes)?
                            .into_iter()
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
