//! Page guards use durable commit footprints, never client-supplied dependencies.
use crate::*;
use serde::{Deserialize, Serialize};

mod dependencies;
mod history;
mod navigation;
pub(crate) use history::coverage;
pub(crate) use history::{envelope, operations, request_value, scrub};

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
    let dependencies = dependencies::closure(&snapshot, &page_ids)?;
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
) -> Result<()> {
    if request.expected_revision > before.revision || commits.len() > 1000 {
        return Err(StorageError::conflict(before.revision));
    }
    let mut guard = dependencies::closure(before, pages)?;
    // New references also need protection; a client cannot attach a stale resource
    // that was not part of its page at read time.
    let after = history::snapshot(before, candidate);
    let old = Candidate::from_snapshot(before);
    for op in &request.operations {
        match op {
            Operation::CreateNode { kind, parent_id: Some(_), .. } if *kind != NodeKind::Page => {},
            Operation::UpdateNode { .. } | Operation::ReparentNode { .. }
            | Operation::ReorderNode { .. } | Operation::DeleteSubtree { .. } => {},
            _ => return Err(unsupported("Scoped batches support existing-page node edits only; use strict mode for shared resources, notes and page structure")),
        }
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
            _ => unreachable!(),
        };
        if props.is_some_and(|p| p.get("flowStart").is_some()) {
            return Err(unsupported("Global flowStart changes require strict mode"));
        }
        for (nodes, target) in [(&old.nodes, id.as_str()), (&candidate.nodes, id.as_str())]
            .into_iter()
            .chain(parent.map(|p| (&candidate.nodes, p)))
        {
            if let Some(page) = dependencies::page(nodes, target) {
                if !pages.contains(&page) {
                    return Err(unsupported(
                        "Operation source and destination pages must be guarded",
                    ));
                }
            }
        }
        let root_frame = |node: &Node, nodes: &BTreeMap<String, Node>| {
            node.kind == NodeKind::Frame
                && node
                    .parent_id
                    .as_ref()
                    .and_then(|id| nodes.get(id))
                    .is_some_and(|n| n.kind == NodeKind::Page)
        };
        match op {
            CreateNode { index, .. }
                if candidate
                    .nodes
                    .get(id)
                    .is_some_and(|n| root_frame(n, &candidate.nodes)) =>
            {
                if index.is_some()
                    || !old
                        .nodes
                        .values()
                        .any(|n| n.parent_id.as_deref() == parent && root_frame(n, &old.nodes))
                {
                    return Err(unsupported("Only appending a root frame after existing roots is supported; first-frame creation requires strict mode"));
                }
            }
            DeleteSubtree { .. } | ReparentNode { .. } | ReorderNode { .. }
                if old.nodes.get(id).is_some_and(|n| root_frame(n, &old.nodes))
                    || candidate
                        .nodes
                        .get(id)
                        .is_some_and(|n| root_frame(n, &candidate.nodes)) =>
            {
                return Err(unsupported(
                    "Root-frame deletion, reparent or reorder requires strict mode",
                ));
            }
            _ => {}
        }
    }
    let footprint = history::footprint(before, candidate)?;
    for key in &footprint {
        if let Some(page) = key.strip_prefix("page:") {
            if !pages.iter().any(|p| p == page) {
                return Err(unsupported("Every affected page, including link cleanup and source/destination pages, must be guarded"));
            }
        } else if !key.starts_with("node:") {
            return Err(unsupported(
                "Scoped shared-resource and note writes are unsupported",
            ));
        }
    }
    // A page reorder/delete is project structure even when its own key is covered.
    let page_map = |nodes: &[Node]| {
        nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Page)
            .map(|n| (n.id.clone(), n.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    if page_map(&before.nodes) != page_map(&after.nodes) {
        return Err(unsupported(
            "Page creation, deletion, reorder or property edits require strict mode",
        ));
    }
    guard.extend(dependencies::closure(&after, pages)?);
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
