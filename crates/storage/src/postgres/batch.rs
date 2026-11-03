use super::*;
use crate::{
    BatchRequest, BatchResult, DryRunResult, StorageError, MAX_BATCH_BYTES, MAX_BATCH_OPS,
};
use serde_json::Value;
use write_through::Committed;

pub(super) enum Outcome {
    /// A new commit carries its rows for the post-commit cache fill; a replay
    /// carries none.
    Commit(BatchResult, Option<Box<Committed>>),
    Preview(DryRunResult),
}

pub(super) fn apply(
    session: &mut Session,
    key: &str,
    request: BatchRequest,
    dry: bool,
) -> Result<Outcome> {
    if dry {
        return preview(session, key, request).map(Outcome::Preview);
    }
    let mut tx = session.client.transaction().map_err(sql)?;
    let mut db = Db::new(&mut tx, &mut session.statements);
    let project = projects::lookup(&mut db, key, true)?;
    let (value, hash) = canonical(&project, &request)?;
    // Read committed: this separate query sees any writer that released our row lock.
    if let Some((mut result, _)) = transaction::replay(
        &mut db,
        &project.meta.project_id,
        &request.idempotency_key,
        &hash,
    )? {
        result.replayed = true;
        return Ok(Outcome::Commit(result, None));
    }
    if project.meta.revision != request.expected_revision {
        return Err(StorageError::conflict(project.meta.revision));
    }
    let identity = (project.generation, project.namespace.clone());
    let (before, binaries, stored) = snapshot::entities_for_write(&mut db, project)?;
    let (candidate, changed_ids) = crate::batch::validated_candidate(&before, &request.operations)?;
    let result = BatchResult {
        project_id: before.project_id.clone(),
        committed_revision: before.revision + 1,
        changed_ids,
        render_status: "pending".into(),
        replayed: false,
    };
    let rows = stored.delta(&candidate)?;
    drop((stored, candidate));
    let written = transaction::persist(
        &mut db,
        &before,
        rows,
        &request.idempotency_key,
        &hash,
        &value["operations"],
        &result,
        None,
    )?;
    tx.commit().map_err(sql)?;
    let revision = result.committed_revision;
    let committed = Committed::new(before, binaries, written, revision, identity);
    Ok(Outcome::Commit(result, Some(Box::new(committed))))
}

/// A preview validates against one repeatable-read snapshot without the
/// project row lock, so it neither waits for nor blocks writers. It applies
/// the commit path's request checks in the same order. A reused key with a
/// different canonical request still fails; a matching key still previews.
fn preview(session: &mut Session, key: &str, request: BatchRequest) -> Result<DryRunResult> {
    let (mut tx, statements) = session.read_only()?;
    let mut db = Db::new(&mut tx, statements);
    let project = projects::lookup(&mut db, key, false)?;
    let (_, hash) = canonical(&project, &request)?;
    transaction::replay(
        &mut db,
        &project.meta.project_id,
        &request.idempotency_key,
        &hash,
    )?;
    if project.meta.revision != request.expected_revision {
        return Err(StorageError::conflict(project.meta.revision));
    }
    let (before, _) = snapshot::entities_with_binaries(&mut db, project)?;
    let (_, changed_ids) = crate::batch::validated_candidate(&before, &request.operations)?;
    Ok(DryRunResult {
        project_id: before.project_id,
        dry_run: true,
        base_revision: before.revision,
        changed_ids,
        diagnostics: vec![],
    })
}

/// Request checks shared by commits and previews: the request's project and
/// key, the operation count and the canonical size. Returns the request value
/// and its canonical hash.
fn canonical(project: &projects::ProjectRow, request: &BatchRequest) -> Result<(Value, String)> {
    transaction::request_ids(
        &project.meta.project_id,
        &request.project_id,
        &request.idempotency_key,
    )?;
    if request.operations.is_empty() || request.operations.len() > MAX_BATCH_OPS {
        return Err(StorageError::validation(
            "operations",
            "Batch requires 1–500 operations",
        ));
    }
    let value = serde_json::to_value(request)?;
    let bytes = serde_json::to_vec(&crate::canonical_json(&value))?;
    if bytes.len() > MAX_BATCH_BYTES {
        return Err(StorageError::validation(
            "operations",
            "Batch exceeds 2 MiB",
        ));
    }
    Ok((value, crate::hash(&bytes)))
}
