use super::*;
use crate::{BatchResult, Snapshot, StorageError};
use ::postgres::GenericClient;
use serde_json::Value;

const REPLAY: &str =
    "SELECT request_hash,response,import_asset FROM idempotency WHERE project_id=$1 AND key=$2";
/// One round trip. Every data-modifying CTE runs exactly once to completion,
/// in the same snapshot; the deleted and upserted entity keys never overlap.
const WRITE: &str = "WITH deleted AS (DELETE FROM entities e USING UNNEST($2::text[],$3::text[]) AS d(kind,id) WHERE e.project_id=$1 AND e.kind=d.kind AND e.id=d.id), \
upserted AS (INSERT INTO entities(project_id,kind,id,data) SELECT $1,d.kind,d.id,d.data FROM UNNEST($4::text[],$5::text[],$6::text[]) AS d(kind,id,data) ON CONFLICT(project_id,kind,id) DO UPDATE SET data=excluded.data), \
revised AS (UPDATE projects SET revision=$7 WHERE project_id=$1), \
committed AS (INSERT INTO commits(project_id,revision,base_revision,batch_id,operations,changed_ids) VALUES($1,$7,$8,$9,$10,$11)) \
INSERT INTO idempotency(project_id,key,request_hash,response,import_asset) VALUES($1,$9,$12,$13,$14)";

pub(super) fn request_ids(project: &str, request_project: &str, key: &str) -> Result<()> {
    if project != request_project {
        return Err(StorageError::validation(
            "projectId",
            "Request project ID does not match this project",
        ));
    }
    if !crate::validate::valid_id(key) {
        return Err(StorageError::validation(
            "idempotencyKey",
            "Idempotency key must contain 1–128 ASCII letters, digits, underscores or hyphens",
        ));
    }
    Ok(())
}

pub(super) fn replay<C: GenericClient>(
    tx: &mut Db<'_, C>,
    project: &str,
    key: &str,
    hash: &str,
) -> Result<Option<(BatchResult, Option<crate::Asset>)>> {
    let row = tx.query_opt(REPLAY, &[&project, &key])?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.get::<_, String>(0) != hash {
        return Err(StorageError::new(
            "IDEMPOTENCY_KEY_REUSED",
            "Idempotency key was already used for a different canonical request",
        ));
    }
    let result: String = row.get(1);
    let asset: Option<String> = row.get(2);
    Ok(Some((
        serde_json::from_str(&result)?,
        asset
            .map(|value| serde_json::from_str(&value))
            .transpose()?,
    )))
}

/// The entity rows `persist` deleted and upserted, with the exact stored text.
pub(super) struct Written {
    pub kinds: Vec<String>,
    pub ids: Vec<String>,
    pub payloads: Vec<String>,
    pub removed_kinds: Vec<String>,
    pub removed_ids: Vec<String>,
}

/// Writes one commit's entity rows, revision, history and replay response in
/// a single statement inside the caller's transaction. `rows` comes from
/// `Stored::delta`; its deleted and upserted keys are disjoint.
#[allow(clippy::too_many_arguments)]
pub(super) fn persist<C: GenericClient>(
    tx: &mut Db<'_, C>,
    before: &Snapshot,
    rows: Written,
    key: &str,
    hash: &str,
    operations: &Value,
    result: &BatchResult,
    imported: Option<&crate::Asset>,
) -> Result<Written> {
    let revision = i64::try_from(result.committed_revision)
        .map_err(|_| StorageError::new("STORAGE_FAILED", "Project revision limit reached"))?;
    tx.execute(
        WRITE,
        &[
            &before.project_id,
            &rows.removed_kinds,
            &rows.removed_ids,
            &rows.kinds,
            &rows.ids,
            &rows.payloads,
            &revision,
            &(before.revision as i64),
            &key,
            &serde_json::to_string(operations)?,
            &serde_json::to_string(&result.changed_ids)?,
            &hash,
            &serde_json::to_string(result)?,
            &imported.map(serde_json::to_string).transpose()?,
        ],
    )?;
    Ok(rows)
}
