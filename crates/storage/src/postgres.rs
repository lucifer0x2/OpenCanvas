//! PostgreSQL persistence for the complete authored product domain.
//! Operations run on a bounded pool of reusable, dedicated blocking workers.
use crate::{Result, SnapshotCache};
use std::{sync::Arc, time::Duration};

mod adapter;
mod batch;
mod config;
mod failure;
#[cfg(test)]
mod failure_tests;
mod history;
mod pool;
mod projects;
mod resources;
mod schema;
mod snapshot;
mod statements;
#[cfg(test)]
mod tests;
mod transaction;
mod write_through;
pub use config::PostgresConfig;
use failure::{sql, unavailable};
use statements::{Db, Session};

/// A cloneable adapter. Construction initializes only its versioned SQL schema;
/// project creation remains an explicit domain operation.
#[derive(Clone)]
pub struct PostgresBackend {
    pool: Arc<pool::Pool>,
    namespace: Arc<str>,
    cache: Option<SnapshotCache>,
}

impl PostgresBackend {
    pub fn connect(url: &str) -> Result<Self> {
        Self::with_config(PostgresConfig::new(url))
    }

    pub fn with_config(config: PostgresConfig) -> Result<Self> {
        let pool = Arc::new(pool::Pool::new(config)?);
        let namespace = pool.run(|session| schema::initialize(&mut session.client))?;
        Ok(Self {
            pool,
            namespace: namespace.into(),
            cache: None,
        })
    }

    /// Durable database identity for an opt-in snapshot cache.
    pub fn cache_namespace(&self) -> &str {
        &self.namespace
    }

    /// Cache reads are always preceded by a successful authoritative SQL probe.
    pub fn with_cache(mut self, cache: SnapshotCache) -> Result<Self> {
        if cache.namespace() != self.cache_namespace() {
            return Err(crate::StorageError::new(
                "INVALID_CACHE_CONFIG",
                "Snapshot cache namespace does not match this database",
            ));
        }
        self.cache = Some(cache);
        Ok(self)
    }
}

fn milliseconds(value: Duration) -> u64 {
    value.as_millis().clamp(1, u32::MAX as u128) as u64
}
