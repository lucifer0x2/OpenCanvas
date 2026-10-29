//! `get_changes`: what changed after a revision, read from the durable commit log and classified
//! against the current snapshot. Pages are cut by commit count and bytes, never mid-commit.
pub(crate) mod conflict;
mod history;
use crate::{
    error::{ApiError, Result},
    state::AppState,
};
use opencanvas_storage::{Commit, Snapshot};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;

const MAX_LIMIT: usize = 100;
const MAX_BUDGET: usize = 65_536;
/// Response groups for changed IDs; an ID absent from the current snapshot is `removed`.
const GROUPS: [&str; 7] = [
    "nodes", "tokens", "styles", "assets", "fonts", "notes", "removed",
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ChangesRequest {
    project_id: String,
    since_revision: u64,
    limit: Option<usize>,
    include_operations: Option<bool>,
    budget: Option<usize>,
}

pub(crate) async fn read(state: &AppState, args: Value) -> Result<Value> {
    let req: ChangesRequest = serde_json::from_value(args)?;
    let limit = req.limit.unwrap_or(MAX_LIMIT);
    let budget = req.budget.unwrap_or(MAX_BUDGET);
    if !(1..=MAX_LIMIT).contains(&limit) || !(1024..=MAX_BUDGET).contains(&budget) {
        return Err(ApiError::invalid(
            "limit must be 1–100 and budget 1024–65536",
        ));
    }
    let s = state.snapshot(req.project_id).await?;
    let since = req.since_revision;
    if since > s.revision {
        return Err(reset(
            s.revision,
            "sinceRevision is ahead of the project; refetch snapshot",
        ));
    }
    let log = history::after(state, &s.project_id, since, s.revision, limit).await?;
    if log.gap {
        return Err(reset(
            s.revision,
            "Change history for this range is unavailable; refetch snapshot",
        ));
    }
    page(
        &s,
        since,
        log.commits,
        limit,
        budget,
        req.include_operations == Some(true),
    )
}

fn reset(current: u64, message: &str) -> ApiError {
    let mut error = ApiError::new("RESET_REQUIRED", message);
    error.0.current_revision = Some(current);
    error
}

/// Adds whole commits while the response fits `budget`. The base is measured with the widest
/// values of the fields that change at the end (`truncated:false`, `nextRevision` = current), and
/// every addition is the exact compact-JSON growth, so the final page never exceeds the budget.
fn page(
    s: &Snapshot,
    since: u64,
    commits: Vec<Commit>,
    limit: usize,
    budget: usize,
    operations: bool,
) -> Result<Value> {
    let mut response = json!({
        "schemaVersion":1,"projectId":s.project_id,"fromRevision":since,
        "currentRevision":s.revision,"commits":[],
        "changed":GROUPS.iter().map(|g| (g.to_string(), json!([]))).collect::<serde_json::Map<_,_>>(),
        "truncated":false,"nextRevision":s.revision,"limits":{"commits":limit,"bytes":budget}
    });
    let mut used = len(&response)?;
    if used > budget {
        return Err(too_large());
    }
    let kinds = Kinds::of(s);
    let mut groups: [Vec<&str>; 7] = Default::default();
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    for commit in &commits {
        let entry = entry(commit, operations);
        let mut grow = len(&entry)? + usize::from(!entries.is_empty());
        let mut fresh = Vec::new();
        let mut counts = groups.each_ref().map(Vec::len);
        // `seen` may take IDs of a commit that does not fit; the loop stops there anyway.
        for id in commit
            .changed_ids
            .iter()
            .filter(|id| seen.insert(id.as_str()))
        {
            let group = kinds.group(id);
            grow += len(id)? + usize::from(counts[group] > 0);
            counts[group] += 1;
            fresh.push((group, id.as_str()));
        }
        if used + grow > budget {
            if entries.is_empty() {
                return Err(too_large());
            }
            break;
        }
        used += grow;
        for (group, id) in fresh {
            groups[group].push(id);
        }
        entries.push((commit.revision, entry));
    }
    let next = entries.last().map_or(since, |e| e.0);
    for (name, ids) in GROUPS.iter().zip(groups) {
        response["changed"][*name] = json!(ids);
    }
    response["commits"] = Value::Array(entries.into_iter().map(|e| e.1).collect());
    response["truncated"] = json!(next < s.revision);
    response["nextRevision"] = json!(next);
    Ok(response)
}

fn entry(commit: &Commit, operations: bool) -> Value {
    let mut entry = json!({"revision":commit.revision,"changedIds":commit.changed_ids});
    if operations {
        entry["operations"] = sanitize(commit.operations.clone());
    }
    entry
}

/// Asset imports log the asset record; its storage key is internal, as in `list_assets`.
fn sanitize(mut operations: Value) -> Value {
    for op in operations.as_array_mut().into_iter().flatten() {
        if let Some(asset) = op.get_mut("asset").and_then(Value::as_object_mut) {
            asset.remove("relativeKey");
        }
    }
    operations
}

fn too_large() -> ApiError {
    ApiError::new(
        "CONTEXT_ITEM_TOO_LARGE",
        "One commit exceeds the response budget; omit includeOperations or use a larger budget",
    )
}

fn len<T: serde::Serialize + ?Sized>(value: &T) -> Result<usize> {
    Ok(serde_json::to_vec(value)?.len())
}

/// IDs of every resource kind in the snapshot, to group changed IDs by what they are now.
struct Kinds<'a>([HashSet<&'a str>; 6]);
impl<'a> Kinds<'a> {
    fn of(s: &'a Snapshot) -> Self {
        Self([
            s.nodes.iter().map(|n| n.id.as_str()).collect(),
            s.tokens.iter().map(|t| t.id.as_str()).collect(),
            s.styles.iter().map(|t| t.id.as_str()).collect(),
            s.assets.iter().map(|t| t.id.as_str()).collect(),
            s.fonts.iter().map(|t| t.id.as_str()).collect(),
            s.notes.iter().map(|t| t.id.as_str()).collect(),
        ])
    }
    /// Index into `GROUPS`: the first kind holding `id`, else `removed`.
    fn group(&self, id: &str) -> usize {
        self.0.iter().position(|set| set.contains(id)).unwrap_or(6)
    }
}
