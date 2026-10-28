//! Durable, per-project serialized SQLite storage. No network or render work runs
//! inside a transaction. All public errors are safe for a client response.
mod error;
mod model;
mod props_catalog;
mod props_suggest;
mod validate;
pub use error::{NodeErrorContext, Result, StorageError};
pub use model::*;
pub use props_catalog::{
    allowed_props, prop_spec, required_props, PropScope, PropSpec, NODE_KINDS, PROPS,
};
pub use props_suggest::suggest_prop;
pub use validate::{
    resolved_nodes, resolved_nodes_for_theme, resolved_tokens_for_theme, validate_slug,
};

mod assets;
mod backup;
mod bundle;
mod bundle_history;
mod bundle_import;
mod bundle_validation;
mod db;
mod migration;
mod persistence;
mod project;
mod store;
mod support;
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
