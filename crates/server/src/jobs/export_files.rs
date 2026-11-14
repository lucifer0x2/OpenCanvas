//! Reads of export files for `GET /api/v1/jobs/{job}/files/{index}`. Ready files come from the
//! export store, so they outlive their job record; an expired job answers `EXPORT_EXPIRED` (410)
//! and an unknown one `JOB_NOT_FOUND` (404), both telling the caller to re-run `export_nodes`.
use super::{
    export_store::{ExportFile, Lookup},
    Jobs, Table,
};
use crate::error::{ApiError, Result};
use std::time::Instant;

impl Table {
    /// Drops expired export files and the job records that still list them, so a later
    /// identical call creates a new job instead of returning dead URLs.
    pub(super) fn expire_exports(&mut self) {
        let expired = self.exports.purge(Instant::now());
        self.forget(&expired);
    }

    /// The project that owns a job record or a live or recently expired export.
    pub(super) fn owner(&self, id: &str) -> Option<String> {
        let record = self.entries.iter().find(|e| e.value["jobId"] == id);
        match record.and_then(|e| e.value["projectId"].as_str()) {
            Some(project) => Some(project.to_owned()),
            None => self.exports.project(id).map(str::to_owned),
        }
    }
}

impl Jobs {
    /// One file of a ready export job.
    pub async fn file(&self, id: &str, index: usize) -> Result<ExportFile> {
        let mut table = self.table.lock().await;
        table.expire_exports();
        match table.exports.lookup(id) {
            Lookup::Live(files) => {
                return files.get(index).cloned().ok_or_else(|| {
                    ApiError::new("FILE_NOT_FOUND", "Job has no file at this index")
                })
            }
            Lookup::Expired => {
                return Err(ApiError::new(
                    "EXPORT_EXPIRED",
                    "Export files expired; re-run export_nodes for new URLs",
                ))
            }
            Lookup::Unknown => {}
        }
        let Some(entry) = table.entries.iter().find(|e| e.value["jobId"] == id) else {
            return Err(ApiError::new(
                "JOB_NOT_FOUND",
                "Export job is unknown (or the server restarted); re-run export_nodes for new URLs",
            ));
        };
        if entry.value["status"] != "ready" {
            return Err(ApiError::new(
                "IMAGE_NOT_READY",
                "Export is pending or failed; inspect the job error",
            ));
        }
        Err(ApiError::new(
            "FILE_NOT_FOUND",
            "Job has no file at this index",
        ))
    }
}
