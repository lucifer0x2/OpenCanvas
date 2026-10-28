//! Durable, per-project serialized SQLite storage. No network or render work runs
//! inside a transaction. All public errors are safe for a client response.
mod error;
mod model;
mod validate;
pub use error::{Result, StorageError};
pub use model::*;
pub use validate::{resolved_nodes, validate_slug};

mod assets;
mod db;
mod persistence;
mod project;
mod store;
mod support;
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
