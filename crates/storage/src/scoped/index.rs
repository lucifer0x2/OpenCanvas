//! Owned, revision-local tree topology. One traversal replaces per-node ancestry walks.
use crate::*;
use validate::NodeIndex;

#[derive(Default)]
pub(crate) struct PageIndex {
    pub node_pages: BTreeMap<String, String>,
    pub page_nodes: BTreeMap<String, Vec<String>>,
    pub roots: BTreeMap<String, Vec<String>>,
    pub root_ids: BTreeSet<String>,
}

impl PageIndex {
    pub fn new<'a>(nodes: impl IntoIterator<Item = &'a Node>) -> Self {
        let nodes: Vec<_> = nodes.into_iter().collect();
        let tree = NodeIndex::new(nodes.iter().copied());
        let mut index = Self::default();
        for page in nodes.iter().filter(|n| n.kind == NodeKind::Page) {
            let mut pending = vec![page.id.as_str()];
            let mut members = Vec::new();
            while let Some(id) = pending.pop() {
                if index
                    .node_pages
                    .insert(id.into(), page.id.clone())
                    .is_some()
                {
                    continue;
                }
                members.push(id.into());
                pending.extend(tree.children(id));
            }
            let mut roots: Vec<_> = tree
                .children(&page.id)
                .filter_map(|id| tree.node(id))
                .filter(|n| n.kind == NodeKind::Frame)
                .map(|n| (n.sort_key, n.id.clone()))
                .collect();
            roots.sort();
            index
                .root_ids
                .extend(roots.iter().map(|(_, id)| id.clone()));
            index.roots.insert(
                page.id.clone(),
                roots.into_iter().map(|(_, id)| id).collect(),
            );
            index.page_nodes.insert(page.id.clone(), members);
        }
        index
    }

    pub fn page(&self, id: &str) -> Option<&str> {
        self.node_pages.get(id).map(String::as_str)
    }

    pub fn validate_pages(&self, pages: &[String]) -> Result<()> {
        Self::validate_page_ids(pages)?;
        if pages.iter().any(|p| !self.page_nodes.contains_key(p)) {
            return Err(StorageError::validation(
                "pageIds",
                "Guarded page does not exist",
            ));
        }
        Ok(())
    }

    pub fn validate_page_ids(pages: &[String]) -> Result<()> {
        if pages.is_empty()
            || pages.len() > 100
            || pages.iter().collect::<BTreeSet<_>>().len() != pages.len()
        {
            return Err(StorageError::validation(
                "pageIds",
                "Supply 1–100 distinct existing page IDs",
            ));
        }
        Ok(())
    }
}

/// Reused by scoped checks and durable history. Changed IDs include apply cascades
/// and pruned links; resource import callers retain the full-diff entry point.
pub(crate) struct BatchScope {
    pub before: Arc<PageIndex>,
    pub after: Arc<PageIndex>,
    pub footprint: BTreeSet<String>,
}

impl BatchScope {
    pub fn new(before: &Snapshot, after: &Candidate, changed: &[String]) -> Result<Self> {
        let old = PageIndex::new(&before.nodes);
        let new = PageIndex::new(after.nodes.values());
        let footprint =
            super::history::footprint_indexed(before, after, Some(changed), &old, &new)?;
        Ok(Self {
            before: Arc::new(old),
            after: Arc::new(new),
            footprint,
        })
    }
}
