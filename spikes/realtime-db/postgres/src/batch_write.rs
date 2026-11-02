//! Set-based persistence of an already validated candidate in one exchange.
use crate::db;
use oc_spike_core::{BatchRequest, DomainError, State, VALIDATOR_VERSION};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use tokio_postgres::{types::Type, Transaction};

const PERSIST: &str = r#"
WITH upserted AS (
    INSERT INTO nodes(project_id,node_id,parent_id,kind,sort_key,props)
    SELECT $1, id, "parentId", kind, "sortKey", props
    FROM jsonb_to_recordset($2) AS node(
        id TEXT, "parentId" TEXT, kind TEXT, "sortKey" BIGINT, props JSONB)
    ON CONFLICT(project_id,node_id) DO UPDATE SET
        parent_id=EXCLUDED.parent_id, kind=EXCLUDED.kind,
        sort_key=EXCLUDED.sort_key, props=EXCLUDED.props
), deleted AS (
    DELETE FROM nodes WHERE project_id=$1 AND node_id=ANY($3)
), advanced AS (
    UPDATE projects SET revision=$4,validated_revision=$4,validator_version=$9 WHERE project_id=$1
    RETURNING project_id,revision
), recorded AS (
    INSERT INTO commits(project_id,revision,changed_ids)
    SELECT project_id,revision,$5 FROM advanced RETURNING project_id
), replay AS (
    INSERT INTO idempotency(project_id,request_key,request_hash,response)
    SELECT project_id,$6,$7,$8 FROM recorded RETURNING project_id
)
SELECT pg_notify('oc_spike_changes',project_id) FROM replay
"#;

pub async fn persist(
    tx: &Transaction<'_>,
    request: &BatchRequest,
    next: &State,
    changed_ids: &[String],
    hash: &str,
    response: &Value,
) -> Result<(), DomainError> {
    // Partition once: the two node DML sets must never target the same row.
    // Writable CTEs share a snapshot and have no guaranteed execution order.
    let mut deleted: BTreeSet<_> = changed_ids.iter().map(String::as_str).collect();
    let upserted: Vec<_> = next
        .nodes
        .iter()
        .filter(|node| deleted.remove(node.id.as_str()))
        .collect();
    let upserted = serde_json::to_value(upserted).map_err(db::internal)?;
    let deleted: Vec<_> = deleted.into_iter().collect();
    let revision = i64::try_from(next.revision).map_err(db::internal)?;
    let changed = json!(changed_ids);
    // Deferred composite FK validates the final tree, including reparent/delete.
    // NOTIFY is transactional: PostgreSQL delivers it only after durable COMMIT.
    tx.query_typed_one(
        PERSIST,
        &[
            (&request.project_id, Type::TEXT),
            (&upserted, Type::JSONB),
            (&deleted, Type::TEXT_ARRAY),
            (&revision, Type::INT8),
            (&changed, Type::JSONB),
            (&request.idempotency_key, Type::TEXT),
            (&hash, Type::TEXT),
            (response, Type::JSONB),
            (&VALIDATOR_VERSION, Type::TEXT),
        ],
    )
    .await
    .map_err(db::internal)?;
    Ok(())
}
