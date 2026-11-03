//! Bulk input is data only. It never executes code or divides a request into commits.
use crate::*;
use serde::{Deserialize, Serialize};
use std::io::Read;
mod inbox;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchFileRequest {
    pub project_id: String,
    pub expected_revision: u64,
    pub idempotency_key: String,
    /// An inbox-relative filename, or an absolute filename inside this project's inbox.
    pub path: String,
}

impl BatchFileRequest {
    pub fn with_operations(&self, operations: Value) -> Result<BatchRequest> {
        Ok(BatchRequest {
            project_id: self.project_id.clone(),
            expected_revision: self.expected_revision,
            idempotency_key: self.idempotency_key.clone(),
            operations: serde_json::from_value(operations)?,
        })
    }
}

impl Project {
    /// Kept as JSON until the MCP layer can validate the original operation shapes
    /// against its published schema, before serde fills any optional fields.
    pub fn read_batch_file_operations(&self, request: &BatchFileRequest) -> Result<Value> {
        self.check_paths()?;
        self.request_ids(&request.project_id, &request.idempotency_key)?;
        let file = inbox::open(&self.root.path, &self.path, &request.path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(StorageError::new(
                "PATH_REJECTED",
                "Batch input must be a regular file",
            ));
        }
        if metadata.len() > MAX_BATCH_BYTES as u64 {
            return Err(StorageError::validation("path", "Batch file exceeds 2 MiB"));
        }
        // Check the stream too, so concurrent file growth cannot bypass the bound.
        let mut bytes = Vec::new();
        file.take(MAX_BATCH_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BATCH_BYTES {
            return Err(StorageError::validation("path", "Batch file exceeds 2 MiB"));
        }
        let operations: Value = serde_json::from_slice(&bytes).map_err(|_| {
            StorageError::validation(
                "operations",
                "Batch file must contain a pure JSON operation array",
            )
        })?;
        if !operations
            .as_array()
            .is_some_and(|a| !a.is_empty() && a.len() <= MAX_BATCH_OPS)
        {
            return Err(StorageError::validation(
                "operations",
                "Batch requires a JSON array of 1–500 operations",
            ));
        }
        Ok(operations)
    }
    pub fn apply_batch_file(&self, request: BatchFileRequest) -> Result<BatchResult> {
        let operations = self.read_batch_file_operations(&request)?;
        self.apply_batch(request.with_operations(operations)?)
    }
    pub fn dry_run_batch_file(&self, request: BatchFileRequest) -> Result<DryRunResult> {
        let operations = self.read_batch_file_operations(&request)?;
        self.dry_run_batch(request.with_operations(operations)?)
    }
}
