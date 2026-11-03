//! Shared immutable fixture bytes; production needs an object store and quotas.
use crate::db;
use oc_spike_core::DomainError;
use sha2::{Digest, Sha256};
use tokio_postgres::GenericClient;

pub const MAX_BYTES: usize = 2 * 1024 * 1024;

pub fn invalid(message: impl std::fmt::Display) -> DomainError {
    DomainError::new("VALIDATION_FAILED", message.to_string())
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn verify(id: &str, bytes: &[u8]) -> Result<(), DomainError> {
    if digest(bytes) != id {
        return Err(DomainError::new(
            "INTEGRITY_ERROR",
            "Stored blob hash mismatch",
        ));
    }
    Ok(())
}

pub async fn put<C: GenericClient>(client: &C, bytes: &[u8]) -> Result<String, DomainError> {
    if bytes.len() > MAX_BYTES {
        return Err(invalid("Blob exceeds 2 MiB"));
    }
    let id = digest(bytes);
    client
        .execute(
            "INSERT INTO shared_blobs(blob_id,bytes) VALUES($1,$2) ON CONFLICT DO NOTHING",
            &[&id, &bytes],
        )
        .await
        .map_err(db::internal)?;
    // Also detect external corruption rather than silently accepting a hash collision.
    let stored = get(client, &id).await?;
    if stored != bytes {
        return Err(DomainError::new(
            "INTEGRITY_ERROR",
            "Conflicting immutable blob",
        ));
    }
    Ok(id)
}

pub async fn get<C: GenericClient>(client: &C, id: &str) -> Result<Vec<u8>, DomainError> {
    let row = client
        .query_opt("SELECT bytes FROM shared_blobs WHERE blob_id=$1", &[&id])
        .await
        .map_err(db::internal)?;
    let bytes: Vec<u8> = row
        .ok_or_else(|| DomainError::new("NOT_FOUND", "Blob not found"))?
        .get(0);
    verify(id, &bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_hash_and_corruption() {
        let id = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(digest(b"abc"), id);
        assert!(verify(id, b"abc").is_ok());
        assert_eq!(verify(id, b"abd").unwrap_err().code, "INTEGRITY_ERROR");
    }
}
