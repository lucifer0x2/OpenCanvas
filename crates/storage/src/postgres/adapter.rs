use super::*;
use crate::{
    Asset, BatchFileRequest, BatchRequest, BatchResult, Commit, DryRunResult, Font,
    ImportAssetRequest, ImportAssetResult, PrivacyStatus, ProjectMeta, Snapshot,
    SnapshotCacheIdentity, StorageBackend, StorageError,
};
use serde_json::Value;
use std::sync::Arc;

impl PostgresBackend {
    /// One worker checkout: the probe and, on a miss, the repeatable-read load.
    /// A hit returns the shared snapshot; any owned copy is made by the caller.
    fn read_snapshot(&self, project: &str) -> Result<snapshot::Identified> {
        let key = project.to_owned();
        let cache = self.cache.clone();
        let namespace = self.namespace.clone();
        let read = self.pool.run(move |session| {
            snapshot::read_through(session, &key, cache.as_ref(), &namespace)
        })?;
        let (snapshot, generation, namespace) = match read {
            snapshot::Read::Hit(read) => return Ok(read),
            snapshot::Read::Loaded(read) => read,
        };
        // Fill off the worker, with the identity of the snapshot's own transaction.
        if let Some(cache) = &self.cache {
            if namespace == self.cache_namespace() {
                cache.put(&snapshot, generation);
            }
        }
        Ok((snapshot, generation, namespace))
    }

    /// Write-through runs on the caller after the worker reported a commit, so
    /// assembling the snapshot never holds a pool worker. Replays carry nothing.
    fn fill(&self, committed: Option<Box<write_through::Committed>>) {
        if let Some(committed) = committed {
            committed.fill(self.cache.as_ref());
        }
    }
}

fn identity(
    project_id: String,
    schema_version: u32,
    revision: u64,
    generation: i64,
    namespace: &str,
) -> SnapshotCacheIdentity {
    SnapshotCacheIdentity {
        project_id,
        schema_version,
        revision,
        incarnation: format!("postgres:{generation}:{namespace}"),
        asset_readiness: vec![],
    }
}

impl StorageBackend for PostgresBackend {
    fn viewer_auth(&self) -> Option<&dyn crate::ViewerAuthStore> {
        Some(self)
    }

    fn list_projects(&self) -> Result<Vec<ProjectMeta>> {
        self.pool.run(|session| {
            let mut projects: Vec<ProjectMeta> = session
                .db()
                .query(projects::LIST, &[])?
                .into_iter()
                .map(|row| Ok(projects::decode(row)?.meta))
                .collect::<Result<_>>()?;
            projects.sort_by(|a, b| a.slug.cmp(&b.slug));
            Ok(projects)
        })
    }

    fn create_project(&self, slug: &str, name: &str) -> Result<ProjectMeta> {
        let (slug, name) = (slug.to_owned(), name.to_owned());
        self.pool
            .run(move |session| projects::create(&mut session.client, &slug, &name))
    }

    fn project_meta(&self, key: &str) -> Result<ProjectMeta> {
        let key = key.to_owned();
        self.pool
            .run(move |session| Ok(projects::lookup(&mut session.db(), &key, false)?.meta))
    }

    fn update_project_privacy(
        &self,
        key: &str,
        privacy: PrivacyStatus,
    ) -> Result<(ProjectMeta, bool)> {
        let key = key.to_owned();
        self.pool
            .run(move |session| projects::update_privacy(session, &key, privacy))
    }

    fn snapshot(&self, project: &str) -> Result<Snapshot> {
        // A hit shares the cached Arc; the owned copy is made here, not on a worker.
        self.read_snapshot(project)
            .map(|(snapshot, _, _)| Arc::unwrap_or_clone(snapshot))
    }

    fn snapshot_cache_identity(&self, project: &str) -> Result<Option<SnapshotCacheIdentity>> {
        let key = project.to_owned();
        let row = self
            .pool
            .run(move |session| projects::lookup(&mut session.db(), &key, false))?;
        Ok(Some(identity(
            row.meta.project_id,
            row.meta.schema_version,
            row.meta.revision,
            row.generation,
            &row.namespace,
        )))
    }

    fn snapshot_with_cache_identity(
        &self,
        project: &str,
    ) -> Result<(Arc<Snapshot>, Option<SnapshotCacheIdentity>)> {
        let (snapshot, generation, namespace) = self.read_snapshot(project)?;
        let identity = identity(
            snapshot.project_id.clone(),
            snapshot.schema_version,
            snapshot.revision,
            generation,
            &namespace,
        );
        Ok((snapshot, Some(identity)))
    }

    fn apply_batch(&self, project: &str, request: BatchRequest) -> Result<BatchResult> {
        let key = project.to_owned();
        let (result, committed) =
            self.pool.run(
                move |session| match batch::apply(session, &key, request, false)? {
                    batch::Outcome::Commit(result, committed) => Ok((result, committed)),
                    _ => unreachable!("commit operation"),
                },
            )?;
        self.fill(committed);
        Ok(result)
    }

    fn dry_run_batch(&self, project: &str, request: BatchRequest) -> Result<DryRunResult> {
        let key = project.to_owned();
        self.pool.run(
            move |session| match batch::apply(session, &key, request, true)? {
                batch::Outcome::Preview(result) => Ok(result),
                _ => unreachable!("preview operation"),
            },
        )
    }

    fn commits_after(&self, project: &str, revision: u64, limit: usize) -> Result<Vec<Commit>> {
        let key = project.to_owned();
        self.pool
            .run(move |session| history::after(session, &key, revision, limit))
    }

    fn import_asset(
        &self,
        project: &str,
        request: ImportAssetRequest,
    ) -> Result<ImportAssetResult> {
        let key = project.to_owned();
        let (result, committed) = self
            .pool
            .run(move |session| resources::import(session, &key, request))?;
        self.fill(committed);
        Ok(result)
    }

    fn read_asset(&self, project: &str, id: &str) -> Result<(Asset, Vec<u8>)> {
        let (key, id) = (project.to_owned(), id.to_owned());
        self.pool
            .run(move |session| resources::read_asset(session, &key, &id))
    }

    fn read_font(&self, project: &str, id: &str) -> Result<(Font, String, Vec<u8>)> {
        let (key, id) = (project.to_owned(), id.to_owned());
        self.pool
            .run(move |session| resources::read_font(session, &key, &id))
    }

    fn read_batch_file_operations(
        &self,
        project: &str,
        _request: &BatchFileRequest,
    ) -> Result<Value> {
        self.project_meta(project)?;
        Err(StorageError::new(
            "STAGING_UNSUPPORTED",
            "This storage backend does not provide a staged operations inbox",
        ))
    }
}
