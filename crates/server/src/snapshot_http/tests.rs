use super::{Format, SnapshotRepresentations};
use opencanvas_storage::*;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};
use std::time::Duration;

type Pause = (Sender<()>, Receiver<()>);

#[derive(Default)]
struct Backend {
    revision: AtomicUsize,
    probes: AtomicUsize,
    reads: AtomicUsize,
    advance_after_probe: AtomicBool,
    unavailable: AtomicBool,
    pause: Mutex<Option<Pause>>,
}

fn identity(revision: u64) -> SnapshotCacheIdentity {
    SnapshotCacheIdentity {
        project_id: "project-id".into(),
        schema_version: 4,
        revision,
        incarnation: "fixture-generation".into(),
        asset_readiness: vec![],
    }
}

impl StorageBackend for Backend {
    fn snapshot_cache_identity(&self, _: &str) -> Result<Option<SnapshotCacheIdentity>> {
        self.probes.fetch_add(1, Relaxed);
        if self.unavailable.load(Relaxed) {
            return Err(StorageError::new("STORAGE_UNAVAILABLE", "Offline fixture"));
        }
        let revision = self.revision.load(Relaxed) as u64;
        if self.advance_after_probe.swap(false, Relaxed) {
            self.revision.fetch_add(1, Relaxed);
        }
        Ok(Some(identity(revision)))
    }

    fn snapshot_with_cache_identity(
        &self,
        key: &str,
    ) -> Result<(Arc<Snapshot>, Option<SnapshotCacheIdentity>)> {
        let snapshot = self.snapshot(key)?;
        let observed = identity(snapshot.revision);
        Ok((Arc::new(snapshot), Some(observed)))
    }

    fn snapshot(&self, _: &str) -> Result<Snapshot> {
        self.reads.fetch_add(1, Relaxed);
        let revision = self.revision.load(Relaxed) as u64;
        let pause = self.pause.lock().unwrap().take();
        if let Some((started, resume)) = pause {
            started.send(()).unwrap();
            resume.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        Ok(serde_json::from_value(json!({
            "projectId":"project-id", "schemaVersion":4, "revision":revision,
            "slug":"slug", "name":"Project", "nodes":[], "tokens":[], "styles":[],
            "assets":[], "fonts":[], "notes":[],
            "retention":{"oldestCommitRevision":1,"idempotencyPolicy":"fixture"}
        }))?)
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
fn fill_uses_the_snapshot_identity_and_hot_hits_still_probe() {
    let backend = Arc::new(Backend::default());
    let store = Storage::from_shared(backend.clone());
    let cache = SnapshotRepresentations::new();
    backend.advance_after_probe.store(true, Relaxed);
    let latest = cache
        .read(&store, "slug", Format::Objects, PrivacyStatus::Private)
        .unwrap();
    let hit = cache
        .read(
            &store,
            "project-id",
            Format::Objects,
            PrivacyStatus::Private,
        )
        .unwrap();
    assert_eq!(latest.bytes, hit.bytes);
    assert_eq!(latest.etag, hit.etag);
    assert_eq!(
        serde_json::from_slice::<Value>(&hit.bytes).unwrap()["revision"],
        1
    );
    assert_eq!(backend.reads.load(Relaxed), 1);
    assert_eq!(backend.probes.load(Relaxed), 2);

    backend.unavailable.store(true, Relaxed);
    let error = cache
        .read(&store, "slug", Format::Objects, PrivacyStatus::Private)
        .err()
        .unwrap();
    assert_eq!(error.code, "STORAGE_UNAVAILABLE");
    assert_eq!(backend.reads.load(Relaxed), 1);
}

#[test]
fn a_delayed_old_fill_cannot_replace_a_newer_hit() {
    let backend = Arc::new(Backend::default());
    let store = Storage::from_shared(backend.clone());
    let cache = SnapshotRepresentations::new();
    let (started, wait_for_old_read) = channel();
    let (resume_old_read, resume) = channel();
    *backend.pause.lock().unwrap() = Some((started, resume));
    std::thread::scope(|scope| {
        let old = scope.spawn(|| {
            cache
                .read(&store, "slug", Format::Rows, PrivacyStatus::Private)
                .unwrap()
        });
        wait_for_old_read
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        backend.revision.store(1, Relaxed);
        let current = cache
            .read(&store, "slug", Format::Rows, PrivacyStatus::Private)
            .unwrap();
        resume_old_read.send(()).unwrap();
        assert_ne!(old.join().unwrap().etag, current.etag);
        let hit = cache
            .read(&store, "slug", Format::Rows, PrivacyStatus::Private)
            .unwrap();
        assert_eq!(hit.etag, current.etag);
        assert_eq!(backend.reads.load(Relaxed), 2);
    });
}
