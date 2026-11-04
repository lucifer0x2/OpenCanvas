use crate::{
    error::{ApiError, Result},
    jobs::Jobs,
    snapshot_http::SnapshotRepresentations,
    RenderBackend,
};
use opencanvas_storage::{ProjectHandle, ProjectMeta, Snapshot, Storage};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{broadcast, watch};

#[derive(Clone)]
pub struct Config {
    pub workspace: PathBuf,
    pub base_url: String,
    pub node: String,
    pub render_backend: RenderBackend,
    pub job_timeout: std::time::Duration,
    pub pipeline_identity: String,
}
impl Config {
    pub fn new(workspace: PathBuf, port: u16) -> Self {
        Self::for_backend(workspace, port, RenderBackend::Chromium)
    }

    /// Select the backend before fingerprinting; Rust never probes the browser runtime.
    ///
    /// # Panics
    /// The Rust backend requires the current executable to be readable for cache identity.
    pub fn for_backend(workspace: PathBuf, port: u16, render_backend: RenderBackend) -> Self {
        let pipeline_identity = match render_backend {
            RenderBackend::Chromium => crate::fingerprint::pipeline(&workspace),
            RenderBackend::Rust => crate::fingerprint::native_pipeline(),
        };
        Self {
            workspace,
            base_url: format!("http://127.0.0.1:{port}"),
            node: "node".into(),
            render_backend,
            job_timeout: std::time::Duration::from_secs(45),
            pipeline_identity,
        }
    }
}
#[derive(Clone)]
pub struct AppState {
    pub store: Storage,
    pub config: Arc<Config>,
    pub events: broadcast::Sender<Value>,
    pub jobs: Arc<Jobs>,
    pub(crate) snapshot_representations: SnapshotRepresentations,
    pub(crate) viewer_auth_limits: Arc<crate::viewer_auth::limits::Limits>,
    shutdown: Arc<watch::Sender<bool>>,
}
impl AppState {
    pub fn new(store: impl Into<Storage>, config: Config) -> Self {
        let store = store.into();
        let (events, _) = broadcast::channel(128);
        let config = Arc::new(config);
        let jobs = Jobs::start(config.clone(), events.clone(), store.clone());
        Self {
            store,
            config,
            events,
            jobs,
            snapshot_representations: SnapshotRepresentations::new(),
            viewer_auth_limits: Arc::new(crate::viewer_auth::limits::Limits::new()),
            shutdown: Arc::new(watch::channel(false).0),
        }
    }
    /// Tells long-lived streams to finish. Without it an open event stream holds a graceful
    /// shutdown open for as long as its client stays connected. `send_replace` sets the flag even
    /// while no stream is subscribed, so a stream opened later still sees it.
    pub fn begin_shutdown(&self) {
        self.shutdown.send_replace(true);
    }
    /// Resolves once `begin_shutdown` has been called, immediately if it already was.
    pub async fn shutting_down(&self) {
        let mut receiver = self.shutdown.subscribe();
        let _ = receiver.wait_for(|stop| *stop).await;
    }
    pub async fn db<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(Storage) -> opencanvas_storage::Result<T> + Send + 'static,
    {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || f(store))
            .await
            .map_err(|_| ApiError::new("INTERNAL_ERROR", "Storage task failed"))?
            .map_err(Into::into)
    }
    pub async fn snapshot(&self, id: String) -> Result<Snapshot> {
        self.db(move |s| s.snapshot(&id)).await
    }
    /// Canonical ID, slug and current revision by ID or slug, without loading the design.
    /// Fails with the same codes as `snapshot` (for example `PROJECT_NOT_FOUND`).
    pub async fn project_meta(&self, id: String) -> Result<ProjectMeta> {
        self.db(move |s| s.project_meta(&id)).await
    }
    /// The project's current revision, or `fallback` when it cannot be read.
    pub async fn revision_or(&self, id: String, fallback: u64) -> u64 {
        self.project_meta(id)
            .await
            .map_or(fallback, |meta| meta.revision)
    }
    /// Runs `f` on the backend's project named by id or slug, off the async reactor.
    pub async fn project<T, F>(&self, id: String, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&ProjectHandle) -> opencanvas_storage::Result<T> + Send + 'static,
    {
        self.db(move |s| {
            let project = s.open_project(&id)?;
            f(&project)
        })
        .await
    }
}
