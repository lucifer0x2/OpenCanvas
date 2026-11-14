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
    indexes: &crate::revision_indexes::IndexCache,
) -> Result<Outcome> {
    apply_mode(session, key, request, dry, None, indexes)
}

pub(super) fn apply_mode(
    session: &mut Session,
    key: &str,
    request: BatchRequest,
    dry: bool,
    pages: Option<Vec<String>>,
    indexes: &crate::revision_indexes::IndexCache,
) -> Result<Outcome> {
    if dry {
        return preview(session, key, request, pages, indexes).map(Outcome::Preview);
    }
    let mut tx = session.client.transaction().map_err(sql)?;
    let mut db = Db::new(&mut tx, &mut session.statements);
    let project = projects::lookup(&mut db, key, true)?;
    let (value, hash) = canonical(&project, &request, pages.as_deref())?;
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
    if pages.is_none() && project.meta.revision != request.expected_revision {
        return Err(StorageError::conflict(project.meta.revision));
    }
    let identity = (project.generation, project.namespace.clone());
    let (before, binaries, stored) =
        crate::bench_phase::time("load", || snapshot::entities_for_write(&mut db, project))?;
    let cache_identity = adapter::identity(
        before.project_id.clone(),
        before.schema_version,
        before.revision,
        identity.0,
        &identity.1,
    );
    let old_indexes = indexes.for_snapshot(&before, Some(cache_identity.clone()));
    let (candidate, changed_ids, mut work, next_indexes) =
        crate::batch::validated_candidate_indexed(
            &before,
            &request.operations,
            old_indexes.clone(),
        )?;
    let scope = crate::scoped::BatchScope::indexed(
        &before,
        &candidate,
        &changed_ids,
        old_indexes,
        next_indexes.clone(),
    )?;
    if let Some(pages) = &pages {
        crate::bench_phase::time("scoped.check", || {
            check(
                &mut db, &before, &candidate, &request, pages, &scope, &mut work,
            )
        })?;
    }
    drop(work);
    let result = BatchResult {
        project_id: before.project_id.clone(),
        committed_revision: before.revision + 1,
        changed_ids,
        render_status: "pending".into(),
        replayed: false,
    };
    let rows = crate::bench_phase::time("persist.delta", || stored.delta(&candidate))?;
    let history = crate::bench_phase::time("history.envelope", || {
        crate::scoped::envelope_with_scopes(
            scope.footprint,
            value.clone(),
            value["operations"].clone(),
        )
    })?;
    drop((stored, candidate));
    let written = {
        let _phase = crate::bench_phase::Span::new("persist");
        let written = transaction::persist(
            &mut db,
            &before,
            rows,
            &request.idempotency_key,
            &hash,
            &history,
            &result,
            None,
        )?;
        tx.commit().map_err(sql)?;
        written
    };
    let revision = result.committed_revision;
    let mut next_identity = cache_identity;
    next_identity.revision = revision;
    indexes.publish(next_identity, next_indexes);
    let committed = Committed::new(before, binaries, written, revision, identity);
    Ok(Outcome::Commit(result, Some(Box::new(committed))))
}

/// A preview validates against one repeatable-read snapshot without the
/// project row lock, so it neither waits for nor blocks writers. It applies
/// the commit path's request checks in the same order. A reused key with a
/// different canonical request still fails; a matching key still previews.
fn preview(
    session: &mut Session,
    key: &str,
    request: BatchRequest,
    pages: Option<Vec<String>>,
    indexes: &crate::revision_indexes::IndexCache,
) -> Result<DryRunResult> {
    let (mut tx, statements) = session.read_only()?;
    let mut db = Db::new(&mut tx, statements);
    let project = projects::lookup(&mut db, key, false)?;
    let (_, hash) = canonical(&project, &request, pages.as_deref())?;
    transaction::replay(
        &mut db,
        &project.meta.project_id,
        &request.idempotency_key,
        &hash,
    )?;
    if pages.is_none() && project.meta.revision != request.expected_revision {
        return Err(StorageError::conflict(project.meta.revision));
    }
    let identity = adapter::identity(
        project.meta.project_id.clone(),
        project.meta.schema_version,
        project.meta.revision,
        project.generation,
        &project.namespace,
    );
    let (before, _) = snapshot::entities_with_binaries(&mut db, project)?;
    let old_indexes = indexes.for_snapshot(&before, Some(identity));
    let (candidate, changed_ids, mut work, next_indexes) =
        crate::batch::validated_candidate_indexed(
            &before,
            &request.operations,
            old_indexes.clone(),
        )?;
    if let Some(pages) = &pages {
        let scope = crate::scoped::BatchScope::indexed(
            &before,
            &candidate,
            &changed_ids,
            old_indexes,
            next_indexes,
        )?;
        check(
            &mut db, &before, &candidate, &request, pages, &scope, &mut work,
        )?;
    }
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
fn canonical(
    project: &projects::ProjectRow,
    request: &BatchRequest,
    pages: Option<&[String]>,
) -> Result<(Value, String)> {
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
    let value = crate::scoped::request_value(request, pages)?;
    let bytes = serde_json::to_vec(&crate::canonical_json(&value))?;
    if bytes.len() > MAX_BATCH_BYTES {
        return Err(StorageError::validation(
            "operations",
            "Batch exceeds 2 MiB",
        ));
    }
    Ok((value, crate::hash(&bytes)))
}

fn check<C: ::postgres::GenericClient>(
    db: &mut Db<'_, C>,
    before: &crate::Snapshot,
    candidate: &crate::validate::Candidate,
    request: &BatchRequest,
    pages: &[String],
    scope: &crate::scoped::BatchScope,
    work: &mut crate::validate::ValidationWork,
) -> Result<()> {
    let rows = db.query("SELECT revision,CASE WHEN sum(octet_length(operations)) OVER (ORDER BY revision)<=8388608 THEN operations ELSE 'null' END FROM commits WHERE project_id=$1 AND revision>$2 ORDER BY revision LIMIT 1001",
        &[&before.project_id, &(request.expected_revision as i64)])?;
    let commits = rows
        .into_iter()
        .map(|row| {
            Ok((
                row.get::<_, i64>(0) as u64,
                serde_json::from_str(&row.get::<_, String>(1))?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    crate::scoped::check(before, candidate, request, pages, commits, scope, work)
}
