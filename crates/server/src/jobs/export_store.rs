//! Export files, kept apart from the 32 job records so render and prerender churn cannot evict a
//! download URL. Files stay for `ttl` after their job finishes, within a total byte budget that
//! drops the oldest jobs first. Memory only: a server restart forgets every export.
use super::export::StoredFile;
use axum::body::Bytes;
use serde_json::Value;
use std::{
    collections::VecDeque,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(test)]
#[path = "export_store_tests.rs"]
mod tests;

/// Default lifetime of an export's files.
pub const EXPORT_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// Default total bytes of retained export files; one job holds at most 48 MiB.
pub const EXPORT_BUDGET: usize = 512 * 1024 * 1024;
/// Expired job IDs remembered so their URLs answer `EXPORT_EXPIRED` rather than unknown.
const GONE_CAP: usize = 1024;

/// How long and how many bytes of export files the server keeps.
#[derive(Debug, Clone, Copy)]
pub struct ExportRetention {
    pub ttl: Duration,
    pub max_bytes: usize,
}

impl Default for ExportRetention {
    fn default() -> Self {
        Self {
            ttl: EXPORT_TTL,
            max_bytes: EXPORT_BUDGET,
        }
    }
}

/// One downloadable export file; cloning shares the bytes.
#[derive(Clone)]
pub struct ExportFile {
    pub meta: Value,
    pub bytes: Bytes,
}

struct Stored {
    id: String,
    project: String,
    expires: Instant,
    bytes: usize,
    files: Vec<ExportFile>,
}

/// What a job ID names in the store.
pub(super) enum Lookup<'a> {
    Live(&'a [ExportFile]),
    Expired,
    Unknown,
}

#[derive(Default)]
pub(super) struct ExportStore {
    limits: ExportRetention,
    /// Oldest first; with one `ttl` this is also expiry order.
    live: VecDeque<Stored>,
    bytes: usize,
    /// Expired or evicted `(job, project)` pairs, newest last.
    gone: VecDeque<(String, String)>,
}

impl ExportStore {
    pub(super) fn new(limits: ExportRetention) -> Self {
        Self {
            limits,
            ..Self::default()
        }
    }

    /// Keeps a finished job's files and returns their expiry in Unix seconds and the jobs
    /// evicted to fit the byte budget. The newest job is always kept.
    pub(super) fn insert(
        &mut self,
        id: &str,
        project: &str,
        files: Vec<StoredFile>,
        now: Instant,
    ) -> (u64, Vec<String>) {
        let files: Vec<_> = files
            .into_iter()
            .map(|f| ExportFile {
                meta: f.meta,
                bytes: Bytes::from(f.bytes),
            })
            .collect();
        let bytes = files.iter().map(|f| f.bytes.len()).sum();
        let mut evicted = self.purge(now);
        while !self.live.is_empty() && self.bytes + bytes > self.limits.max_bytes {
            evicted.push(self.drop_oldest());
        }
        self.bytes += bytes;
        self.live.push_back(Stored {
            id: id.into(),
            project: project.into(),
            expires: now + self.limits.ttl,
            bytes,
            files,
        });
        let wall = SystemTime::now() + self.limits.ttl;
        let expires_at = wall.duration_since(UNIX_EPOCH).unwrap_or_default();
        (expires_at.as_secs(), evicted)
    }

    /// Drops every job whose files outlived `ttl` and returns their IDs.
    pub(super) fn purge(&mut self, now: Instant) -> Vec<String> {
        let mut expired = Vec::new();
        while self.live.front().is_some_and(|s| s.expires <= now) {
            expired.push(self.drop_oldest());
        }
        expired
    }

    pub(super) fn lookup(&self, id: &str) -> Lookup<'_> {
        if let Some(stored) = self.live.iter().find(|s| s.id == id) {
            return Lookup::Live(&stored.files);
        }
        if self.gone.iter().any(|(gone, _)| gone == id) {
            return Lookup::Expired;
        }
        Lookup::Unknown
    }

    /// The project of a live or recently expired export, for public-viewer admission.
    pub(super) fn project(&self, id: &str) -> Option<&str> {
        let live = self.live.iter().find(|s| s.id == id).map(|s| &s.project);
        let gone = || self.gone.iter().find(|(g, _)| g == id).map(|(_, p)| p);
        live.or_else(gone).map(String::as_str)
    }

    fn drop_oldest(&mut self) -> String {
        let stored = self.live.pop_front().expect("store is non-empty");
        self.bytes -= stored.bytes;
        if self.gone.len() >= GONE_CAP {
            self.gone.pop_front();
        }
        self.gone.push_back((stored.id.clone(), stored.project));
        stored.id
    }
}
