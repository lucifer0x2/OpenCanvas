use super::{SnapshotCacheIdentity, StorageBackend};
use crate::{
    Asset, BatchFileRequest, BatchRequest, BatchResult, Commit, DryRunResult, Font,
    ImportAssetRequest, ImportAssetResult, PrivacyStatus, ProjectMeta, Result, Snapshot, Store,
};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};
mod inbox;

/// Cloneable application handle. Choose the implementation at the composition root.
#[derive(Clone)]
pub struct Storage {
    backend: Arc<dyn StorageBackend>,
    /// Local directory holding `<slug>/inbox`; see `with_inbox_root`.
    inbox_root: Option<Arc<PathBuf>>,
}

impl Storage {
    pub fn new(backend: impl StorageBackend + 'static) -> Self {
        Self::from_shared(Arc::new(backend))
    }

    pub fn from_shared(backend: Arc<dyn StorageBackend>) -> Self {
        Self {
            backend,
            inbox_root: None,
        }
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectMeta>> {
        self.backend.list_projects()
    }

    pub fn viewer_auth(&self) -> Option<&dyn crate::ViewerAuthStore> {
        self.backend.viewer_auth()
    }

    pub fn project_meta(&self, key: &str) -> Result<ProjectMeta> {
        self.backend.project_meta(key)
    }

    pub fn update_project_privacy(
        &self,
        key: &str,
        privacy: PrivacyStatus,
    ) -> Result<(ProjectMeta, bool)> {
        self.backend.update_project_privacy(key, privacy)
    }

    pub fn create_project(&self, slug: &str, name: &str) -> Result<Arc<ProjectHandle>> {
        let meta = self.backend.create_project(slug, name)?;
        Ok(self.handle(meta.project_id))
    }

    pub fn open_project(&self, key: &str) -> Result<Arc<ProjectHandle>> {
        let meta = self.backend.project_meta(key)?;
        Ok(self.handle(meta.project_id))
    }

    pub fn snapshot(&self, key: &str) -> Result<Snapshot> {
        self.backend.snapshot(key)
    }

    pub fn snapshot_cache_identity(&self, key: &str) -> Result<Option<SnapshotCacheIdentity>> {
        self.backend.snapshot_cache_identity(key)
    }

    pub fn snapshot_with_cache_identity(
        &self,
        key: &str,
    ) -> Result<(Arc<Snapshot>, Option<SnapshotCacheIdentity>)> {
        self.backend.snapshot_with_cache_identity(key)
    }

    fn handle(&self, project_id: String) -> Arc<ProjectHandle> {
        Arc::new(ProjectHandle {
            storage: self.clone(),
            project_id,
        })
    }
}

impl From<Store> for Storage {
    fn from(store: Store) -> Self {
        let root = store.root.path.clone();
        Self::new(super::SqliteBackend::new(store)).with_inbox_root(root)
    }
}

/// Engine-neutral project facade. It retains a canonical ID, never a connection.
pub struct ProjectHandle {
    storage: Storage,
    project_id: String,
}

impl ProjectHandle {
    pub fn apply_scoped_batch(&self, request: crate::ScopedBatchRequest) -> Result<BatchResult> {
        self.storage.backend.apply_scoped_batch(self.id(), request)
    }
    pub fn dry_run_scoped_batch(&self, request: crate::ScopedBatchRequest) -> Result<DryRunResult> {
        self.storage
            .backend
            .dry_run_scoped_batch(self.id(), request)
    }
    pub fn id(&self) -> &str {
        &self.project_id
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        self.storage.backend.snapshot(self.id())
    }

    pub fn apply_batch(&self, request: BatchRequest) -> Result<BatchResult> {
        self.storage.backend.apply_batch(self.id(), request)
    }

    pub fn dry_run_batch(&self, request: BatchRequest) -> Result<DryRunResult> {
        self.storage.backend.dry_run_batch(self.id(), request)
    }

    pub fn commits_after(&self, revision: u64, limit: usize) -> Result<Vec<Commit>> {
        self.storage
            .backend
            .commits_after(self.id(), revision, limit)
    }

    pub fn import_asset(&self, request: ImportAssetRequest) -> Result<ImportAssetResult> {
        self.storage.backend.import_asset(self.id(), request)
    }

    pub fn read_asset(&self, id: &str) -> Result<(Asset, Vec<u8>)> {
        self.storage.backend.read_asset(self.id(), id)
    }

    pub fn read_font(&self, id: &str) -> Result<(Font, String, Vec<u8>)> {
        self.storage.backend.read_font(self.id(), id)
    }

    pub fn read_batch_file_operations(&self, request: &BatchFileRequest) -> Result<Value> {
        self.storage
            .backend
            .read_batch_file_operations(self.id(), request)
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
