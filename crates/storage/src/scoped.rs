//! Page guards use durable commit footprints, never client-supplied dependencies.
use crate::*;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod dependencies;
pub(crate) mod dependency_index;
mod history;
pub(crate) mod index;
mod navigation;
mod notes;
#[cfg(test)]
mod parity;
#[cfg(test)]
mod reference_history;
pub(crate) use history::coverage;
pub(crate) use history::envelope_with_scopes;
pub(crate) use history::{envelope, operations, request_value, scrub};
pub(crate) use index::BatchScope;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScopedBatchRequest {
    pub project_id: String,
    pub expected_revision: u64,
    pub idempotency_key: String,
    pub page_ids: Vec<String>,
    pub operations: Vec<Operation>,
}
impl ScopedBatchRequest {
    pub(crate) fn split(self) -> (BatchRequest, Vec<String>) {
        (
            BatchRequest {
                project_id: self.project_id,
                expected_revision: self.expected_revision,
                idempotency_key: self.idempotency_key,
                operations: self.operations,
            },
            self.page_ids,
        )
    }
}

/// A snapshot and its guard always come from the same storage read.
pub fn edit_context(snapshot: Snapshot, page_ids: Vec<String>) -> Result<Value> {
    let indexes = RevisionIndexes::new(&snapshot.nodes);
    edit_context_indexed(&snapshot, &indexes, page_ids)
}

pub(crate) fn edit_context_indexed(
    snapshot: &Snapshot,
    indexes: &RevisionIndexes,
    page_ids: Vec<String>,
) -> Result<Value> {
    index::PageIndex::validate_page_ids(&page_ids)?;
    let index = indexes.dependencies(|| {
        dependency_index::DependencyIndex::with_snapshot_tree(
            snapshot,
            indexes.pages.clone(),
            Some(&indexes.references),
        )
    })?;
    index.tree.validate_pages(&page_ids)?;
    let dependencies = index.closure(&page_ids)?;
    Ok(
        json!({"projectId":snapshot.project_id,"expectedRevision":snapshot.revision,
        "pageIds":page_ids,"dependencyCount":dependencies.len(),
        "dependencies":dependencies.iter().take(100).collect::<Vec<_>>(),
        "dependenciesTruncated":dependencies.len()>100}),
    )
}

pub(crate) fn check(
    before: &Snapshot,
    candidate: &Candidate,
    request: &BatchRequest,
    pages: &[String],
    commits: Vec<(u64, Value)>,
    scope: &BatchScope,
    work: &mut validate::ValidationWork,
) -> Result<()> {
    if request.expected_revision > before.revision || commits.len() > 1000 {
        return Err(StorageError::conflict(before.revision));
    }
    // A guarded page exists already or is appended by this batch.
    index::PageIndex::validate_page_ids(pages)?;
    let existed = |p: &String| scope.before.page_nodes.contains_key(p);
    if !pages
        .iter()
        .all(|p| existed(p) || scope.after.page_nodes.contains_key(p))
    {
        return Err(StorageError::validation(
            "pageIds",
            "Guarded page does not exist",
        ));
    }
    notes::check(before, candidate, &request.operations, pages, scope)?;
    for op in &request.operations {
        use Operation::*;
        let (id, parent, props) = match op {
            CreateNode {
                id,
                parent_id,
                props,
                ..
            } => (id, parent_id.as_deref(), Some(props)),
            UpdateNode { id, props } => (id, None, Some(props)),
            ReparentNode { id, parent_id, .. } => (id, Some(parent_id.as_str()), None),
            ReorderNode { id, .. } | DeleteSubtree { id } => (id, None, None),
            CreateNote { .. } | ReplyNote { .. } | UpdateNote { .. } | DeleteNote { .. } => {
                continue
            }
            _ => {
                return Err(unsupported(
                    "Token and style writes require strict mode (omit pageIds)",
                ))
            }
        };
        if props.is_some_and(|p| p.get("flowStart").is_some()) {
            return Err(unsupported("Global flowStart changes require strict mode"));
        }
        for (index, target) in [(&scope.before, id.as_str()), (&scope.after, id.as_str())]
            .into_iter()
            .chain(parent.map(|p| (&scope.after, p)))
        {
            if let Some(page) = index.page(target) {
                if !pages.iter().any(|p| p == page) {
                    return Err(unsupported(
                        "Operation source and destination pages must be guarded",
                    ));
                }
            }
        }
        match op {
            CreateNode { index: Some(_), .. } if scope.after.root_ids.contains(id) => {
                return Err(unsupported(
                    "Append root frames without an explicit index in scoped mode",
                ));
            }
            DeleteSubtree { .. } | ReparentNode { .. } | ReorderNode { .. }
                if scope.before.root_ids.contains(id) || scope.after.root_ids.contains(id) =>
            {
                return Err(unsupported(
                    "Root-frame deletion, reparent or reorder requires strict mode",
                ));
            }
            _ => {}
        }
    }
    if scope.footprint.contains("*") {
        return Err(unsupported("Changing the default page or first root frame, page order, page deletion or page properties requires strict mode"));
    }
    for key in &scope.footprint {
        if let Some(page) = key.strip_prefix("page:") {
            if !pages.iter().any(|p| p == page) {
                return Err(unsupported("Every affected page, including link cleanup, note anchors and source/destination pages, must be guarded"));
            }
        } else if !key.starts_with("node:") && !key.starts_with("note:") {
            return Err(unsupported("Scoped shared-resource writes are unsupported"));
        }
    }
    // At the current revision there is no concurrent history to intersect. All
    // page/restriction/effective-coverage checks above still run in the same order.
    let mut guard = BTreeSet::new();
    if request.expected_revision != before.revision {
        // Pages appended by this batch have no old closure; their new closure
        // still guards the page key against concurrent create/delete.
        let old_pages: Vec<_> = pages.iter().filter(|p| existed(p)).cloned().collect();
        if !old_pages.is_empty() {
            let old = scope.old_indexes.dependencies(|| {
                dependency_index::DependencyIndex::with_snapshot_tree(
                    before,
                    scope.before.clone(),
                    Some(&scope.old_indexes.references),
                )
            })?;
            guard = old.closure(&old_pages)?;
        }
        let new = scope.new_indexes.dependencies(|| {
            dependency_index::DependencyIndex::from_candidate(
                candidate,
                work.expanded.take(),
                scope.after.clone(),
                Some(&scope.new_indexes.references),
            )
        })?;
        // New references need protection too; a client cannot attach a resource
        // changed since its read even if the old page did not reference it.
        guard.extend(new.closure(pages)?);
    }
    let mut expected = request.expected_revision + 1;
    for (revision, value) in commits {
        if revision != expected || value["scopeVersion"] != 1 {
            return Err(StorageError::conflict(before.revision));
        }
        let scopes = history::coverage(&value, &before.project_id, revision - 1)
            .map_err(|_| StorageError::conflict(before.revision))?;
        if scopes.iter().any(|key| key == "*" || guard.contains(key)) {
            return Err(StorageError::conflict(before.revision));
        }
        expected += 1;
    }
    if expected != before.revision + 1 {
        return Err(StorageError::conflict(before.revision));
    }
    Ok(())
}

fn unsupported(message: &str) -> StorageError {
    StorageError::new("SCOPED_OPERATION_UNSUPPORTED", message)
}
