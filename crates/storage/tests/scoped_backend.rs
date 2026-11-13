//! A custom backend's cache identity cannot opt it into trusted scene reads.
use opencanvas_storage::*;
use serde_json::{json, Value};
use std::sync::Arc;

struct Custom(Snapshot);
impl StorageBackend for Custom {
    fn snapshot(&self, _: &str) -> Result<Snapshot> {
        Ok(self.0.clone())
    }
    fn snapshot_with_cache_identity(
        &self,
        _: &str,
    ) -> Result<(Arc<Snapshot>, Option<SnapshotCacheIdentity>)> {
        Ok((
            Arc::new(self.0.clone()),
            Some(SnapshotCacheIdentity {
                project_id: self.0.project_id.clone(),
                revision: self.0.revision,
                schema_version: self.0.schema_version,
                incarnation: "custom:stable".into(),
                asset_readiness: vec![],
            }),
        ))
    }
    fn list_projects(&self) -> Result<Vec<ProjectMeta>> {
        unreachable!()
    }
    fn create_project(&self, _: &str, _: &str) -> Result<ProjectMeta> {
        unreachable!()
    }
    fn project_meta(&self, _: &str) -> Result<ProjectMeta> {
        unreachable!()
    }
    fn apply_batch(&self, _: &str, _: BatchRequest) -> Result<BatchResult> {
        unreachable!()
    }
    fn dry_run_batch(&self, _: &str, _: BatchRequest) -> Result<DryRunResult> {
        unreachable!()
    }
    fn commits_after(&self, _: &str, _: u64, _: usize) -> Result<Vec<Commit>> {
        unreachable!()
    }
    fn import_asset(&self, _: &str, _: ImportAssetRequest) -> Result<ImportAssetResult> {
        unreachable!()
    }
    fn read_asset(&self, _: &str, _: &str) -> Result<(Asset, Vec<u8>)> {
        unreachable!()
    }
    fn read_font(&self, _: &str, _: &str) -> Result<(Font, String, Vec<u8>)> {
        unreachable!()
    }
    fn read_batch_file_operations(&self, _: &str, _: &BatchFileRequest) -> Result<Value> {
        unreachable!()
    }
}

#[test]
fn custom_identity_keeps_conservative_full_validation_and_legacy_optout() {
    let snapshot: Snapshot = serde_json::from_value(json!({
        "schemaVersion":4,"projectId":"custom","slug":"custom","name":"Custom","revision":1,
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"lifetime"},
        "nodes":[{"id":"page","kind":"page","parentId":null,"sortKey":0,"props":{}},
            {"id":"bad","kind":"rectangle","parentId":"page","sortKey":1,"props":{"unsupported":true}}],
        "tokens":[],"styles":[],"assets":[],"fonts":[],"notes":[]
    })).unwrap();
    let reference = resolved_expanded_nodes(&snapshot, "default").unwrap_err();
    let storage = Storage::new(Custom(snapshot));
    let shared = storage.shared_snapshot("custom").unwrap();
    assert!(shared.trusted_scoped().unwrap().is_none());
    assert_eq!(
        serde_json::to_value(shared.scoped().err().unwrap()).unwrap(),
        serde_json::to_value(reference).unwrap()
    );
    let again = storage.shared_snapshot("custom").unwrap();
    assert!(!Arc::ptr_eq(&shared.indexes, &again.indexes));
}
