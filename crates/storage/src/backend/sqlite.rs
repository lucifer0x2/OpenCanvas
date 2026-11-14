use super::{pins::ProjectPins, SnapshotCacheIdentity, StorageBackend};
use crate::{
    Asset, BatchFileRequest, BatchRequest, BatchResult, Commit, DryRunResult, Font,
    ImportAssetRequest, ImportAssetResult, PrivacyStatus, ProjectMeta, Result, Snapshot, Store,
};
use serde_json::Value;
use std::sync::Arc;

pub(crate) mod identity;
mod viewer_auth;

/// Default adapter. All domain writes and validation use the existing SQLite
/// implementation; bounded project pins preserve its hot writer/registry path.
pub struct SqliteBackend {
    store: Store,
    pins: ProjectPins,
}

impl SqliteBackend {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            pins: ProjectPins::default(),
        }
    }
}

impl StorageBackend for SqliteBackend {
    fn apply_scoped_batch(
        &self,
        project: &str,
        request: crate::ScopedBatchRequest,
    ) -> Result<BatchResult> {
        self.pins
            .open(&self.store, project)?
            .apply_scoped_batch(request)
    }
    fn dry_run_scoped_batch(
        &self,
        project: &str,
        request: crate::ScopedBatchRequest,
    ) -> Result<DryRunResult> {
        self.pins
            .open(&self.store, project)?
            .dry_run_scoped_batch(request)
    }
    fn viewer_auth(&self) -> Option<&dyn crate::ViewerAuthStore> {
        Some(self)
    }

    fn list_projects(&self) -> Result<Vec<ProjectMeta>> {
        self.store.list_projects()
    }

    fn create_project(&self, slug: &str, name: &str) -> Result<ProjectMeta> {
        let project = self.store.create_project(slug, name)?;
        let meta = crate::read_meta(&project.reader()?)?;
        self.pins.pin(slug, &project);
        Ok(meta)
    }

    fn project_meta(&self, key: &str) -> Result<ProjectMeta> {
        self.pins.metadata(&self.store, key)
    }

    fn update_project_privacy(
        &self,
        key: &str,
        privacy: PrivacyStatus,
    ) -> Result<(ProjectMeta, bool)> {
        self.pins.open(&self.store, key)?.update_privacy(privacy)
    }

    fn snapshot(&self, project: &str) -> Result<Snapshot> {
        self.pins
            .read(&self.store, project, |handle| {
                let snapshot = handle.snapshot()?;
                Ok((identity::snapshot_meta(&snapshot), snapshot))
            })
            .map(|(_, snapshot)| snapshot)
    }

    fn snapshot_cache_identity(&self, project: &str) -> Result<Option<SnapshotCacheIdentity>> {
        self.pins
            .read(&self.store, project, identity::probe)
            .map(|(_, identity)| identity)
    }

    fn shared_snapshot(&self, project: &str) -> Result<crate::SharedSnapshot> {
        let (snapshot, identity) = self.snapshot_with_cache_identity(project)?;
        let indexes = self.store.root.indexes.for_snapshot(&snapshot, identity);
        Ok(crate::SharedSnapshot::committed(snapshot, indexes))
    }

    fn snapshot_with_cache_identity(
        &self,
        project: &str,
    ) -> Result<(Arc<Snapshot>, Option<SnapshotCacheIdentity>)> {
        self.pins
            .read(&self.store, project, identity::snapshot)
            .map(|(_, (snapshot, identity))| (Arc::new(snapshot), identity))
    }

    fn apply_batch(&self, project: &str, request: BatchRequest) -> Result<BatchResult> {
        self.pins.open(&self.store, project)?.apply_batch(request)
    }

    fn dry_run_batch(&self, project: &str, request: BatchRequest) -> Result<DryRunResult> {
        self.pins.open(&self.store, project)?.dry_run_batch(request)
    }

    fn commits_after(&self, project: &str, revision: u64, limit: usize) -> Result<Vec<Commit>> {
        self.pins
            .open(&self.store, project)?
            .commits_after(revision, limit)
    }

    fn import_asset(
        &self,
        project: &str,
        request: ImportAssetRequest,
    ) -> Result<ImportAssetResult> {
        self.pins.open(&self.store, project)?.import_asset(request)
    }

    fn read_asset(&self, project: &str, id: &str) -> Result<(Asset, Vec<u8>)> {
        self.pins.open(&self.store, project)?.read_asset(id)
    }

    fn read_font(&self, project: &str, id: &str) -> Result<(Font, String, Vec<u8>)> {
        self.pins.open(&self.store, project)?.read_font(id)
    }

    fn read_batch_file_operations(
        &self,
        project: &str,
        request: &BatchFileRequest,
    ) -> Result<Value> {
        self.pins
            .open(&self.store, project)?
            .read_batch_file_operations(request)
    }
}
