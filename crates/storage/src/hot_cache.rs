//! Process-local, disposable snapshots. PostgreSQL supplies the identity for every lookup.

use crate::Snapshot;
use moka::{ops::compute::Op, sync::Cache};
use std::{io, sync::Arc, time::Duration};

const TTL: Duration = Duration::from_secs(3_600);
const MAX_WEIGHT: u64 = 64 * 1024 * 1024;
const MAX_ENTRY_WEIGHT: u32 = 16 * 1024 * 1024;

#[derive(Clone)]
pub struct SnapshotCache {
    namespace: String,
    entries: Cache<String, CachedSnapshot>,
    max_entry_weight: u32,
}

#[derive(Clone)]
struct CachedSnapshot {
    generation: i64,
    snapshot: Arc<Snapshot>,
    weight: u32,
}

impl SnapshotCache {
    /// Bind an optional cache to the durable namespace returned by PostgreSQL.
    /// Clones share entries, capacity and expiration.
    pub fn new(database_namespace: impl Into<String>) -> Self {
        Self::with_limits(database_namespace.into(), TTL, MAX_WEIGHT, MAX_ENTRY_WEIGHT)
    }

    fn with_limits(namespace: String, ttl: Duration, capacity: u64, max_entry_weight: u32) -> Self {
        let entries = Cache::builder()
            .max_capacity(capacity)
            .weigher(|_: &String, entry: &CachedSnapshot| entry.weight)
            .time_to_live(ttl)
            .build();
        Self {
            namespace,
            entries,
            max_entry_weight,
        }
    }

    pub(crate) fn namespace(&self) -> &str {
        &self.namespace
    }

    /// A hit shares the resident snapshot; callers that need ownership clone it
    /// themselves, off any pool worker.
    pub(crate) fn get(
        &self,
        project: &str,
        generation: i64,
        revision: u64,
    ) -> Option<Arc<Snapshot>> {
        let entry = self.entries.get(project)?;
        (entry.generation == generation && entry.snapshot.revision == revision)
            .then_some(entry.snapshot)
    }

    /// Fill with the identity from the same transaction as the snapshot, or
    /// from the committing write transaction. Keep a newer resident entry when
    /// concurrent reads and commits finish out of order.
    pub(crate) fn put(&self, snapshot: &Arc<Snapshot>, generation: i64) {
        let Some(weight) = snapshot_weight(snapshot, self.max_entry_weight) else {
            return;
        };
        self.entries
            .entry_by_ref(&snapshot.project_id)
            .and_compute_with(|current| {
                if current.is_some_and(|entry| {
                    let value = entry.value();
                    (value.generation, value.snapshot.revision) >= (generation, snapshot.revision)
                }) {
                    return Op::Nop;
                }
                Op::Put(CachedSnapshot {
                    generation,
                    snapshot: Arc::clone(snapshot),
                    weight,
                })
            });
    }
}

// Count JSON bytes as a proxy for relative snapshot size without storing JSON
// or allocating a serialized buffer. Abort the count at the admission limit.
fn snapshot_weight(snapshot: &Snapshot, limit: u32) -> Option<u32> {
    struct Remaining(u32);

    impl io::Write for Remaining {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 = u32::try_from(bytes.len())
                .ok()
                .and_then(|count| self.0.checked_sub(count))
                .ok_or_else(|| io::Error::other("snapshot exceeds cache admission limit"))?;
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let mut remaining = Remaining(limit);
    serde_json::to_writer(&mut remaining, snapshot).ok()?;
    Some(limit - remaining.0)
}

#[cfg(test)]
mod tests;
