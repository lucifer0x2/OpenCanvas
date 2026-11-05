//! Durable, per-project serialized SQLite storage. No network or render work runs
//! inside a transaction. All public errors are safe for a client response.
pub use opencanvas_scene_input::*;

mod assets;
mod backend;
mod batch;
mod batch_file;
pub use backend::{ProjectHandle, SnapshotCacheIdentity, SqliteBackend, Storage, StorageBackend};
pub use batch::DryRunResult;
pub use batch_file::BatchFileRequest;
mod backup;
mod bundle;
mod bundle_history;
mod bundle_import;
mod bundle_validation;
mod db;
mod fonts;
mod hot_cache;
pub use hot_cache::SnapshotCache;
mod migration;
mod persistence;
mod postgres;
mod project;
pub use postgres::{PostgresBackend, PostgresConfig};
mod store;
mod support;
mod viewer_auth;
pub use bundle::{BundleFile, BundleManifest, ProjectBundle, MAX_BUNDLE_BYTES};
use db::*;
use fs2::FileExt;
pub use project::Project;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
};
use store::Root;
pub use store::Store;
use support::*;
use validate::Candidate;
pub use viewer_auth::{ViewerAuthConfig, ViewerAuthStore, ViewerSession};
