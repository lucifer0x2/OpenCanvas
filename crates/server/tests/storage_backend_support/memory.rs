//! A deliberately narrow in-memory fixture. No Store, Project, driver, or filesystem access.
mod mutations;
use opencanvas_storage::*;
use serde_json::{json, Value};
use std::sync::Mutex;

#[derive(Default)]
pub struct MemoryBackend {
    state: Mutex<State>,
    calls: Mutex<Vec<&'static str>>,
}
#[derive(Default)]
struct State {
    projects: Vec<Document>,
}
struct Document {
    snapshot: Snapshot,
    commits: Vec<Commit>,
    replays: Vec<(String, Value, BatchResult)>,
}
impl Document {
    fn meta(&self) -> ProjectMeta {
        let s = &self.snapshot;
        ProjectMeta {
            project_id: s.project_id.clone(),
            slug: s.slug.clone(),
            name: s.name.clone(),
            schema_version: s.schema_version,
            revision: s.revision,
        }
    }
}
impl State {
    fn get(&self, key: &str) -> Result<&Document> {
        self.projects
            .iter()
            .find(|p| p.snapshot.project_id == key || p.snapshot.slug == key)
            .ok_or_else(not_found)
    }
    fn get_mut(&mut self, key: &str) -> Result<&mut Document> {
        self.projects
            .iter_mut()
            .find(|p| p.snapshot.project_id == key || p.snapshot.slug == key)
            .ok_or_else(not_found)
    }
}
fn not_found() -> StorageError {
    StorageError::new("PROJECT_NOT_FOUND", "Memory fixture project is absent")
}
fn unsupported() -> StorageError {
    StorageError::new(
        "FIXTURE_UNSUPPORTED",
        "Operation outside the in-memory fixture scope",
    )
}
pub const ASSET_BYTES: &[u8] = b"independent-backend-asset";
pub const FONT_BYTES: &[u8] = b"independent-backend-font";
impl MemoryBackend {
    fn trace(&self, operation: &'static str) {
        self.calls.lock().unwrap().push(operation);
    }
    pub fn calls(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap().clone()
    }
}
impl StorageBackend for MemoryBackend {
    fn list_projects(&self) -> Result<Vec<ProjectMeta>> {
        self.trace("list");
        Ok(self
            .state
            .lock()
            .unwrap()
            .projects
            .iter()
            .map(Document::meta)
            .collect())
    }
    fn create_project(&self, slug: &str, name: &str) -> Result<ProjectMeta> {
        self.trace("create");
        let mut state = self.state.lock().unwrap();
        let snapshot = serde_json::from_value(json!({
            "schemaVersion":1,"projectId":format!("memory_{}", state.projects.len()),"slug":slug,"name":name,"revision":0,
            "nodes":[],"tokens":[],"styles":[],"notes":[],
            "assets":[{"id":"memory_asset","contentHash":"fixture","mediaType":"image/png","byteSize":ASSET_BYTES.len(),"relativeKey":"opaque","width":1,"height":1,"status":"ready"},
                      {"id":"memory_font_asset","contentHash":"fixture-font","mediaType":"font/ttf","byteSize":FONT_BYTES.len(),"relativeKey":"opaque-font","width":null,"height":null,"status":"ready"}],
            "fonts":[{"id":"memory_font","family":"Memory Fixture","weight":400,"style":"normal","assetId":"memory_font_asset","bundledId":null,"contentHash":"fixture-font","license":null,"status":"ready"}],
            "retention":{"oldestCommitRevision":1,"idempotencyPolicy":"all"}
        }))?;
        let document = Document {
            snapshot,
            commits: vec![],
            replays: vec![],
        };
        let meta = document.meta();
        state.projects.push(document);
        Ok(meta)
    }
    fn project_meta(&self, key: &str) -> Result<ProjectMeta> {
        self.trace("meta");
        Ok(self.state.lock().unwrap().get(key)?.meta())
    }
    fn snapshot(&self, key: &str) -> Result<Snapshot> {
        self.trace("snapshot");
        Ok(self.state.lock().unwrap().get(key)?.snapshot.clone())
    }
    fn apply_batch(&self, key: &str, request: BatchRequest) -> Result<BatchResult> {
        self.trace("apply");
        mutations::apply(self.state.lock().unwrap().get_mut(key)?, request)
    }
    fn dry_run_batch(&self, _: &str, _: BatchRequest) -> Result<DryRunResult> {
        self.trace("dry-run");
        Err(unsupported())
    }
    fn commits_after(&self, key: &str, revision: u64, limit: usize) -> Result<Vec<Commit>> {
        self.trace("commits");
        Ok(self
            .state
            .lock()
            .unwrap()
            .get(key)?
            .commits
            .iter()
            .filter(|c| c.revision > revision)
            .take(limit)
            .cloned()
            .collect())
    }
    fn import_asset(&self, _: &str, _: ImportAssetRequest) -> Result<ImportAssetResult> {
        Err(unsupported())
    }
    fn read_asset(&self, key: &str, id: &str) -> Result<(Asset, Vec<u8>)> {
        self.trace("asset");
        let state = self.state.lock().unwrap();
        let asset = state
            .get(key)?
            .snapshot
            .assets
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(unsupported)?;
        Ok((asset.clone(), ASSET_BYTES.into()))
    }
    fn read_font(&self, key: &str, id: &str) -> Result<(Font, String, Vec<u8>)> {
        self.trace("font");
        let state = self.state.lock().unwrap();
        let font = state
            .get(key)?
            .snapshot
            .fonts
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(unsupported)?;
        Ok((font.clone(), "font/ttf".into(), FONT_BYTES.into()))
    }
    fn read_batch_file_operations(&self, _: &str, _: &BatchFileRequest) -> Result<Value> {
        Err(unsupported())
    }
}
