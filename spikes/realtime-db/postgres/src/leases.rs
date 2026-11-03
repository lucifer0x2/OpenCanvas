//! Database-time leases and monotonically increasing attempts fence stale workers.
use crate::{blobs, db, jobs};
use oc_spike_core::DomainError;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Claim {
    worker_id: String,
    lease_ms: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Completion {
    worker_id: String,
    attempt: i64,
    bytes: Vec<u8>,
}

pub async fn claim(url: &str, request: Claim) -> Result<Value, DomainError> {
    if request.worker_id.is_empty()
        || request.worker_id.len() > 128
        || !(100..=60_000).contains(&request.lease_ms)
    {
        return Err(blobs::invalid("Invalid worker ID or lease (100..60000 ms)"));
    }
    let mut client = db::connect(url).await?;
    let tx = client.transaction().await.map_err(db::internal)?;
    let row = tx.query_opt("UPDATE render_jobs SET status='running',attempt=attempt+1,worker_id=$1,lease_expires_at=clock_timestamp()+($2::bigint * interval '1 millisecond') WHERE job_id=(SELECT job_id FROM render_jobs WHERE status='pending' OR (status='running' AND lease_expires_at<=clock_timestamp()) ORDER BY job_id FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING job_id", &[&request.worker_id,&request.lease_ms]).await.map_err(db::internal)?;
    let job = match row {
        Some(row) => jobs::get(&tx, &row.get::<_, String>(0)).await?,
        None => Value::Null,
    };
    tx.commit().await.map_err(db::internal)?;
    Ok(json!({"job":job}))
}

pub async fn complete(url: &str, id: &str, request: Completion) -> Result<Value, DomainError> {
    let mut client = db::connect(url).await?;
    let tx = client.transaction().await.map_err(db::internal)?;
    // Update holds the row lock through artifact insertion and commit. A concurrent
    // claimant cannot change the attempt after this ownership check succeeds.
    let changed = tx.execute("UPDATE render_jobs SET status='completed' WHERE job_id=$1 AND status='running' AND attempt=$2 AND worker_id=$3 AND lease_expires_at>clock_timestamp()", &[&id,&request.attempt,&request.worker_id]).await.map_err(db::internal)?;
    if changed == 0 {
        return Err(DomainError::new(
            "STALE_ATTEMPT",
            "Lease expired or attempt superseded",
        ));
    }
    let artifact_id = blobs::put(&tx, &request.bytes).await?;
    tx.execute(
        "UPDATE render_jobs SET artifact_id=$2 WHERE job_id=$1",
        &[&id, &artifact_id],
    )
    .await
    .map_err(db::internal)?;
    let result = jobs::get(&tx, id).await?;
    tx.commit().await.map_err(db::internal)?;
    Ok(result)
}
