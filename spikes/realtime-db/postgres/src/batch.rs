use crate::{batch_write, db, delta, profile::BatchProfile};
use oc_spike_core::{canonical_hash, BatchRequest, BatchResult, DomainError};
use serde_json::{json, Value};
use tokio_postgres::types::Type;

pub async fn execute(url: &str, request: &BatchRequest) -> Result<Value, DomainError> {
    let mut profile = BatchProfile::new(request.operations.len());
    execute_profiled(url, request, &mut profile).await
}

pub(crate) async fn execute_profiled(
    url: &str,
    request: &BatchRequest,
    profile: &mut BatchProfile,
) -> Result<Value, DomainError> {
    let timer = profile.timer();
    let hash = canonical_hash(request)?;
    profile.record("hash", timer);
    let timer = profile.timer();
    let mut client = db::connect(url).await?;
    profile.record("pool", timer);
    let timer = profile.timer();
    let tx = client.transaction().await.map_err(db::internal)?;
    profile.record("begin", timer);
    let timer = profile.timer();
    let row = tx
        .query_typed_opt(
            "SELECT revision,validated_revision,validator_version FROM projects WHERE project_id=$1 FOR UPDATE",
            &[(&request.project_id, Type::TEXT)],
        )
        .await
        .map_err(db::internal)?;
    profile.record("lock", timer);
    let row = row.ok_or_else(|| DomainError::new("PROJECT_NOT_FOUND", "Project does not exist"))?;
    let revision = row.get::<_, i64>(0) as u64;
    let validated_revision: Option<i64> = row.get(1);
    let validator_version: Option<String> = row.get(2);
    let timer = profile.timer();
    let replay = tx
        // READ COMMITTED takes a new statement snapshot after the lock wait.
        // Combining this read with FOR UPDATE can miss the writer's replay row.
        .query_typed_opt(
            "SELECT request_hash,response FROM idempotency WHERE project_id=$1 AND request_key=$2",
            &[
                (&request.project_id, Type::TEXT),
                (&request.idempotency_key, Type::TEXT),
            ],
        )
        .await
        .map_err(db::internal)?;
    profile.record("replay", timer);
    if let Some(row) = replay {
        if row.get::<_, String>(0) != hash {
            return Err(DomainError::new(
                "IDEMPOTENCY_KEY_REUSED",
                "Key used with a different request",
            ));
        }
        if !request.dry_run {
            let mut response: Value = row.get(1);
            response["replayed"] = json!(true);
            let timer = profile.timer();
            tx.commit().await.map_err(db::internal)?;
            profile.record("commit", timer);
            profile.finish("replayed", 0);
            return Ok(response);
        }
    }
    // Keep replay/collision precedence above this guard. A stale command cannot
    // reach candidate validation, so it need not load the project's node tree.
    if revision != request.expected_revision {
        return Err(DomainError {
            code: "REVISION_CONFLICT".into(),
            message: "Expected revision does not match".into(),
            current_revision: Some(revision),
        });
    }
    let (next, changed_ids) = delta::candidate(
        &tx,
        request,
        revision,
        validated_revision,
        validator_version.as_deref(),
        profile,
    )
    .await?;
    if request.dry_run {
        tx.rollback().await.map_err(db::internal)?;
        profile.finish("dryRun", next.nodes.len());
        return Ok(
            json!({"projectId":request.project_id,"dryRun":true,"baseRevision":revision,"changedIds":changed_ids}),
        );
    }
    let timer = profile.timer();
    let response = serde_json::to_value(BatchResult {
        project_id: request.project_id.clone(),
        committed_revision: next.revision,
        changed_ids: changed_ids.clone(),
        replayed: false,
    })
    .map_err(db::internal)?;
    batch_write::persist(&tx, request, &next, &changed_ids, &hash, &response).await?;
    profile.record("persist", timer);
    let timer = profile.timer();
    tx.commit().await.map_err(db::internal)?;
    profile.record("commit", timer);
    profile.finish("committed", next.nodes.len());
    Ok(response)
}
