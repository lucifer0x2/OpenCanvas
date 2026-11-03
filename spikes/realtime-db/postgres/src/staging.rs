//! Shared, bounded JSON upload IDs replace Pod-local batch inbox paths.
use crate::{batch, blobs, db};
use oc_spike_core::{canonical_hash, BatchRequest, DomainError};
use serde_json::{json, Value};

pub async fn upload(url: &str, bytes: &[u8]) -> Result<Value, DomainError> {
    if bytes.len() > blobs::MAX_BYTES {
        return Err(blobs::invalid("Upload exceeds 2 MiB"));
    }
    let request: BatchRequest = serde_json::from_slice(bytes).map_err(blobs::invalid)?;
    canonical_hash(&request)?; // Core enforces <=500 ops and request semantics.
    let mut client = db::connect(url).await?;
    let tx = client.transaction().await.map_err(db::internal)?;
    let id = blobs::put(&tx, bytes).await?;
    db::revision(&tx, &request.project_id).await?;
    tx.execute(
        "INSERT INTO staged_batches(upload_id,project_id) VALUES($1,$2) ON CONFLICT DO NOTHING",
        &[&id, &request.project_id],
    )
    .await
    .map_err(db::internal)?;
    tx.commit().await.map_err(db::internal)?;
    Ok(json!({"uploadId":id,"sha256":id,"byteLength":bytes.len()}))
}

pub async fn execute(url: &str, project_id: &str, id: &str) -> Result<Value, DomainError> {
    let bytes = {
        let client = db::connect(url).await?;
        if client
            .query_opt(
                "SELECT 1 FROM staged_batches WHERE upload_id=$1 AND project_id=$2",
                &[&id, &project_id],
            )
            .await
            .map_err(db::internal)?
            .is_none()
        {
            return Err(DomainError::new("NOT_FOUND", "Upload not found"));
        }
        blobs::get(&*client, id).await?
    };
    let request: BatchRequest = serde_json::from_slice(&bytes).map_err(blobs::invalid)?;
    batch::execute(url, &request).await
}
