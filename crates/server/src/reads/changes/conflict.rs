//! `changedSince` on `REVISION_CONFLICT`: which IDs the commits the caller lost to touched, so an
//! agent can rebase without re-reading the design. Best effort: if the log cannot be read the
//! storage error is returned unchanged. Code, status, message and `currentRevision` never change.
use super::history;
use crate::{error::ApiError, state::AppState};
use serde_json::json;
use std::collections::HashSet;

/// At most this many distinct changed IDs, in commit order.
pub(crate) const MAX_IDS: usize = 200;
/// At most this many commits are read to build them.
const MAX_COMMITS: usize = 100;

/// Passes `result` through, enriching a revision conflict on `project` for a request that
/// expected `expected`.
pub(crate) async fn checked<T>(
    state: &AppState,
    project: &str,
    expected: u64,
    result: Result<T, ApiError>,
) -> Result<T, ApiError> {
    match result {
        Err(error) if error.0.code == "REVISION_CONFLICT" => {
            Err(enrich(state, project, expected, error).await)
        }
        other => other,
    }
}

async fn enrich(state: &AppState, project: &str, expected: u64, error: ApiError) -> ApiError {
    // An expectedRevision ahead of the project has no "changes since"; keep the plain conflict.
    let Some(current) = error.0.current_revision.filter(|c| *c > expected) else {
        return error;
    };
    let Ok(log) = history::after(state, project, expected, current, MAX_COMMITS).await else {
        return error;
    };
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    let mut overflow = false;
    'commits: for commit in &log.commits {
        for id in &commit.changed_ids {
            if seen.insert(id.as_str()) {
                if ids.len() == MAX_IDS {
                    overflow = true;
                    break 'commits;
                }
                ids.push(id.as_str());
            }
        }
    }
    let reached = log.commits.last().is_some_and(|c| c.revision == current);
    let truncated = overflow || log.gap || !reached;
    error.with_detail(
        "changedSince",
        json!({"fromRevision":expected,"toRevision":current,"changedIds":ids,"truncated":truncated}),
    )
}
