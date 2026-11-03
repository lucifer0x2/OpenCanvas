//! Immutable pinned render-input fixture; no rendering engine runs in this spike.
use crate::{blobs, db};
use oc_spike_core::DomainError;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_postgres::GenericClient;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Submit {
    project_id: String,
    revision: u64,
    pipeline_fingerprint: String,
    #[serde(default)]
    resource_ids: Vec<String>,
    render_params: Value,
}

pub async fn submit(url: &str, request: Submit) -> Result<Value, DomainError> {
    if request.pipeline_fingerprint.is_empty()
        || request.pipeline_fingerprint.len() > 128
        || request.resource_ids.len() > 100
        || !request.render_params.is_object()
    {
        return Err(blobs::invalid(
            "Invalid pipeline, resource list or render params",
        ));
    }
    let snapshot = db::snapshot(url, &request.project_id).await?;
    if snapshot.revision != request.revision {
        return Err(DomainError::new(
            "REVISION_CONFLICT",
            "Job input revision changed",
        ));
    }
    let input = json!({"snapshot":snapshot,"pipelineFingerprint":request.pipeline_fingerprint,
        "resourceIds":request.resource_ids,"renderParams":request.render_params});
    // serde_json's default map ordering is canonical; no preserve_order in this crate.
    let encoded = serde_json::to_vec(&input).map_err(db::internal)?;
    if encoded.len() > blobs::MAX_BYTES {
        return Err(blobs::invalid("Pinned input exceeds 2 MiB"));
    }
    let id = blobs::digest(&encoded);
    let mut client = db::connect(url).await?;
    let tx = client.transaction().await.map_err(db::internal)?;
    for resource_id in &request.resource_ids {
        if tx
            .query_opt(
                "SELECT 1 FROM resources WHERE resource_id=$1",
                &[&resource_id],
            )
            .await
            .map_err(db::internal)?
            .is_none()
        {
            return Err(DomainError::new("NOT_FOUND", "Input resource not found"));
        }
        blobs::get(&tx, resource_id).await?;
    }
    let revision = i64::try_from(request.revision).map_err(blobs::invalid)?;
    tx.execute("INSERT INTO render_jobs(job_id,project_id,revision,pipeline_fingerprint,input) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING", &[&id,&request.project_id,&revision,&request.pipeline_fingerprint,&input]).await.map_err(db::internal)?;
    let result = get(&tx, &id).await?;
    tx.commit().await.map_err(db::internal)?;
    Ok(result)
}

pub async fn get<C: GenericClient>(client: &C, id: &str) -> Result<Value, DomainError> {
    let row = client.query_opt("SELECT jsonb_build_object('jobId',job_id,'projectId',project_id,'revision',revision,'pipelineFingerprint',pipeline_fingerprint,'input',input,'status',status,'attempt',attempt,'workerId',worker_id,'leaseExpiresAt',lease_expires_at::text,'artifactId',artifact_id) FROM render_jobs WHERE job_id=$1", &[&id]).await.map_err(db::internal)?;
    row.map(|r| r.get(0))
        .ok_or_else(|| DomainError::new("NOT_FOUND", "Job not found"))
}
