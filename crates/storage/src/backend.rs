//! Engine-neutral, blocking storage operations used by the application runtime.
//!
//! Call these operations on a blocking worker. Implementations must bound waits and
//! release transactions before returning; callers may render or stream the result.
use crate::{
    Asset, BatchFileRequest, BatchRequest, BatchResult, Commit, DryRunResult, Font,
    ImportAssetRequest, ImportAssetResult, PrivacyStatus, ProjectMeta, Result, Snapshot,
    StorageError,
};
use serde_json::Value;
use std::sync::Arc;

mod handle;
mod pins;
pub(crate) mod sqlite;
pub use handle::{ProjectHandle, Storage};
pub use sqlite::SqliteBackend;

/// The identity of one immutable snapshot representation within a Storage.
/// A backend that opts in must change this identity whenever any field in the
/// snapshot changes, including changes outside a normal revisioned commit.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SnapshotCacheIdentity {
    pub project_id: String,
    pub schema_version: u32,
    pub revision: u64,
    pub incarnation: String,
    pub asset_readiness: Vec<bool>,
}

/// One domain contract for a complete storage implementation.
///
/// Project lookup accepts canonical IDs or slugs. Returned metadata identifies
/// the canonical project; requests must remain scoped to that ID. Backends own
/// validation, atomic rollback, revision conflicts, durable commits, idempotent
/// replay/key collisions, and exact resource bytes. Dry runs perform the same
/// validation without changing state. Errors use the shared client-safe codes.
///
/// Domain operations require explicit implementations. Snapshot cache identity
/// is optional. Engine-specific maintenance such as WAL checkpoints and SQLite
/// bundles remains on `Store` and `Project`, outside this contract.
pub trait StorageBackend: Send + Sync {
    fn apply_scoped_batch(
        &self,
        _project: &str,
        _request: crate::ScopedBatchRequest,
    ) -> Result<BatchResult> {
        Err(StorageError::new(
            "STORAGE_UNSUPPORTED",
            "This backend does not support scoped concurrency",
        ))
    }
    fn dry_run_scoped_batch(
        &self,
        _project: &str,
        _request: crate::ScopedBatchRequest,
    ) -> Result<DryRunResult> {
        Err(StorageError::new(
            "STORAGE_UNSUPPORTED",
            "This backend does not support scoped concurrency",
        ))
    }
    /// Optional workspace-wide viewer authentication for embedders with custom backends.
    fn viewer_auth(&self) -> Option<&dyn crate::ViewerAuthStore> {
        None
    }

    fn list_projects(&self) -> Result<Vec<ProjectMeta>>;
    fn create_project(&self, slug: &str, name: &str) -> Result<ProjectMeta>;
    fn project_meta(&self, key: &str) -> Result<ProjectMeta>;
    /// Persist access policy outside the authored snapshot and report whether it changed.
    fn update_project_privacy(
        &self,
        _key: &str,
        _privacy: PrivacyStatus,
    ) -> Result<(ProjectMeta, bool)> {
        Err(StorageError::new(
            "STORAGE_UNSUPPORTED",
            "This storage backend does not support updating project privacy",
        ))
    }
    fn snapshot(&self, project: &str) -> Result<Snapshot>;
    /// Custom backends without authoritative identity never share derived state.
    fn shared_snapshot(&self, project: &str) -> Result<crate::SharedSnapshot> {
        let (snapshot, _) = self.snapshot_with_cache_identity(project)?;
        let indexes = Arc::new(crate::RevisionIndexes::new(&snapshot.nodes));
        Ok(crate::SharedSnapshot { snapshot, indexes })
    }
    /// Authoritative lookup by ID or slug. A source error must propagate.
    /// Backends that cannot provide a cheap, reliable probe return None.
    fn snapshot_cache_identity(&self, _project: &str) -> Result<Option<SnapshotCacheIdentity>> {
        Ok(None)
    }
    /// A cache fill must use the identity of the same read as the snapshot.
    /// None makes the snapshot eligible to serve without populating the cache.
    /// The snapshot is shared so read-only callers, such as response encoding,
    /// avoid copying a backend-cached value.
    fn snapshot_with_cache_identity(
        &self,
        project: &str,
    ) -> Result<(Arc<Snapshot>, Option<SnapshotCacheIdentity>)> {
        self.snapshot(project)
            .map(|snapshot| (Arc::new(snapshot), None))
    }
    fn apply_batch(&self, project: &str, request: BatchRequest) -> Result<BatchResult>;
    fn dry_run_batch(&self, project: &str, request: BatchRequest) -> Result<DryRunResult>;
    fn commits_after(&self, project: &str, revision: u64, limit: usize) -> Result<Vec<Commit>>;
    fn import_asset(&self, project: &str, request: ImportAssetRequest)
        -> Result<ImportAssetResult>;
    fn read_asset(&self, project: &str, id: &str) -> Result<(Asset, Vec<u8>)>;
    fn read_font(&self, project: &str, id: &str) -> Result<(Font, String, Vec<u8>)>;
    /// Reads data-only operations from backend-owned staging. The legacy path is
    /// interpreted and contained by the backend, never by a shared local-FS layer.
    /// A backend without staging must return an explicit unsupported error.
    fn read_batch_file_operations(
        &self,
        project: &str,
        request: &BatchFileRequest,
    ) -> Result<Value>;
}
