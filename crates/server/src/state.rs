use crate::{
    error::{ApiError, Result},
    jobs::Jobs,
};
use opencanvas_storage::{Snapshot, Store};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{broadcast, watch};

#[derive(Clone)]
pub struct Config {
    pub workspace: PathBuf,
    pub base_url: String,
    pub node: String,
    pub job_timeout: std::time::Duration,
    pub pipeline_identity: String,
}
impl Config {
    pub fn new(workspace: PathBuf, port: u16) -> Self {
        let pipeline_identity = crate::fingerprint::pipeline(&workspace);
        Self {
            workspace,
            base_url: format!("http://127.0.0.1:{port}"),
            node: "node".into(),
            job_timeout: std::time::Duration::from_secs(45),
            pipeline_identity,
        }
    }
}
#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub config: Arc<Config>,
    pub events: broadcast::Sender<Value>,
    pub jobs: Arc<Jobs>,
    shutdown: Arc<watch::Sender<bool>>,
}
impl AppState {
    pub fn new(store: Store, config: Config) -> Self {
        let (events, _) = broadcast::channel(128);
        let config = Arc::new(config);
        let jobs = Jobs::start(config.clone(), events.clone());
        Self {
            store,
            config,
            events,
            jobs,
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
        F: FnOnce(Store) -> opencanvas_storage::Result<T> + Send + 'static,
    {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || f(store))
            .await
            .map_err(|_| ApiError::new("INTERNAL_ERROR", "Storage task failed"))?
            .map_err(Into::into)
    }
    pub async fn snapshot(&self, id: String) -> Result<Snapshot> {
        self.db(move |s| s.open_project(&id)?.snapshot()).await
    }
}
