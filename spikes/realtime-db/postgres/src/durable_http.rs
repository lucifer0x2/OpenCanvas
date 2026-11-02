//! JSON-only fixture API: uploaded bytes are data, never evaluated or executed.
use crate::{blobs, db, jobs, leases, staging};
use oc_spike_core::DomainError;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};

fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DomainError> {
    serde_json::from_slice(bytes).map_err(blobs::invalid)
}

pub async fn route(
    url: &str,
    method: &str,
    path: &str,
    body: &[u8],
) -> Option<Result<Value, DomainError>> {
    Some(match (method, path) {
        ("POST", "/resources") => resource_upload(url, body).await,
        ("GET", path) if path.starts_with("/resources/") => resource_get(url, &path[11..]).await,
        ("POST", "/uploads") => staging::upload(url, body).await,
        ("POST", "/batch-file") => staged_execute(url, body).await,
        ("POST", "/jobs") => match parse(body) {
            Ok(request) => jobs::submit(url, request).await,
            Err(error) => Err(error),
        },
        ("POST", "/jobs/claim") => match parse(body) {
            Ok(request) => leases::claim(url, request).await,
            Err(error) => Err(error),
        },
        ("POST", path) if path.starts_with("/jobs/") && path.ends_with("/complete") => {
            match parse(body) {
                Ok(request) => leases::complete(url, &path[6..path.len() - 9], request).await,
                Err(error) => Err(error),
            }
        }
        ("GET", path) if path.starts_with("/jobs/") => job_get(url, &path[6..]).await,
        ("GET", path) if path.starts_with("/artifacts/") => artifact_get(url, &path[11..]).await,
        _ => return None,
    })
}

async fn staged_execute(url: &str, body: &[u8]) -> Result<Value, DomainError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Request {
        project_id: String,
        upload_id: String,
    }
    let request: Request = parse(body)?;
    staging::execute(url, &request.project_id, &request.upload_id).await
}

async fn resource_upload(url: &str, body: &[u8]) -> Result<Value, DomainError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        kind: String,
        bytes: Vec<u8>,
        sha256: Option<String>,
    }
    let request: Request = parse(body)?;
    if !matches!(request.kind.as_str(), "asset" | "font") {
        return Err(blobs::invalid("Expected asset or font kind"));
    }
    if request
        .sha256
        .as_ref()
        .is_some_and(|id| *id != blobs::digest(&request.bytes))
    {
        return Err(blobs::invalid("Uploaded bytes do not match claimed hash"));
    }
    let mut client = db::connect(url).await?;
    let tx = client.transaction().await.map_err(db::internal)?;
    let id = blobs::put(&tx, &request.bytes).await?;
    tx.execute(
        "INSERT INTO resources(resource_id,kind) VALUES($1,$2) ON CONFLICT DO NOTHING",
        &[&id, &request.kind],
    )
    .await
    .map_err(db::internal)?;
    tx.commit().await.map_err(db::internal)?;
    Ok(json!({"resourceId":id,"sha256":id,"kind":request.kind,"bytes":request.bytes}))
}

async fn resource_get(url: &str, id: &str) -> Result<Value, DomainError> {
    let client = db::connect(url).await?;
    let rows = client
        .query(
            "SELECT kind FROM resources WHERE resource_id=$1 ORDER BY kind",
            &[&id],
        )
        .await
        .map_err(db::internal)?;
    if rows.is_empty() {
        return Err(DomainError::new("NOT_FOUND", "Resource not found"));
    }
    let kinds: Vec<String> = rows.iter().map(|row| row.get(0)).collect();
    let bytes = blobs::get(&*client, id).await?;
    Ok(json!({"resourceId":id,"sha256":id,"kind":kinds[0],"kinds":kinds,"bytes":bytes}))
}

async fn job_get(url: &str, id: &str) -> Result<Value, DomainError> {
    let client = db::connect(url).await?;
    jobs::get(&*client, id).await
}

async fn artifact_get(url: &str, id: &str) -> Result<Value, DomainError> {
    let client = db::connect(url).await?;
    if client
        .query_opt(
            "SELECT 1 FROM render_jobs WHERE artifact_id=$1 LIMIT 1",
            &[&id],
        )
        .await
        .map_err(db::internal)?
        .is_none()
    {
        return Err(DomainError::new("NOT_FOUND", "Artifact not found"));
    }
    let bytes = blobs::get(&*client, id).await?;
    Ok(json!({"artifactId":id,"sha256":id,"bytes":bytes}))
}
