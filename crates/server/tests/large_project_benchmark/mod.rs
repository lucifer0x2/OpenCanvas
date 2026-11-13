pub mod cell;
pub mod commits;
pub mod mcp;
pub mod scale;
pub mod seed;
pub mod snapshots;
pub mod source;
pub mod stats;

use opencanvas_storage::{PostgresBackend, PostgresConfig, SnapshotCache, Storage, Store};
use std::{path::Path, sync::Arc, time::Duration};

/// `primary` is the production-shaped storage (PostgreSQL with Moka); `uncached`
/// reads the same database without any snapshot cache. SQLite has no such cache.
#[derive(Clone)]
pub struct Backends {
    pub primary: Storage,
    pub uncached: Storage,
    pub moka: bool,
    _root: Option<Arc<tempfile::TempDir>>,
}

impl Backends {
    pub fn sqlite(root: Arc<tempfile::TempDir>) -> Self {
        let storage = Storage::from(Store::open(root.path()).unwrap());
        Self {
            primary: storage.clone(),
            uncached: storage,
            moka: false,
            _root: Some(root),
        }
    }

    /// Production pool size; the 15 s statement and 30 s operation timeouts are
    /// raised so the largest scales report a time instead of STORAGE_UNAVAILABLE.
    pub fn postgres(url: &str) -> Self {
        let config = || {
            let mut config = PostgresConfig::new(url);
            config.statement_timeout = Duration::from_secs(600);
            config.operation_timeout = Duration::from_secs(600);
            config
        };
        let cached = PostgresBackend::with_config(config()).unwrap();
        let cache = SnapshotCache::new(cached.cache_namespace());
        Self {
            primary: Storage::new(cached.with_cache(cache).unwrap()),
            uncached: Storage::new(PostgresBackend::with_config(config()).unwrap()),
            moka: true,
            _root: None,
        }
    }
}

pub fn temp_root() -> Arc<tempfile::TempDir> {
    let parent = std::env::var("TASK24_SQLITE_PARENT")
        .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
    Arc::new(
        tempfile::Builder::new()
            .prefix("task24-sqlite.")
            .tempdir_in(Path::new(&parent))
            .unwrap(),
    )
}
