//! Keeps opened projects alive for the life of the server. Storage's registry holds only `Weak`
//! handles, so without a strong one every read resolved a `p_<uuid>` id by opening every project
//! database in the data root, then reopened the writer and ran migrations. Opening still goes
//! through `Store::open_project`, so its root and project path checks run on every call; a pin
//! only turns its registry lookup into a hit, for the id and the slug alike.
use opencanvas_storage::{Project, Result, StorageError, Store};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

/// Keys pinned before the map is cleared (one id and at most one slug per project). Bounds open
/// SQLite writers in a data root with many projects; a cleared project reopens on its next use.
const MAX_KEYS: usize = 128;

#[derive(Default)]
pub(crate) struct ProjectPins(Mutex<HashMap<String, Arc<Project>>>);

impl ProjectPins {
    /// A failure on a pinned key drops that pin and retries once, so a project removed from
    /// disk reports exactly what an unpinned open reports.
    pub fn open(&self, store: &Store, key: &str) -> Result<Arc<Project>> {
        let project = match store.open_project(key) {
            Err(_) if self.forget(key) => store.open_project(key)?,
            other => other?,
        };
        self.pin(key, &project);
        Ok(project)
    }

    /// `snapshot` of the project named by `key`, rejecting a pinned handle whose directory was
    /// replaced out of band (the database there belongs to another project).
    pub fn snapshot(&self, store: &Store, key: &str) -> Result<opencanvas_storage::Snapshot> {
        let snapshot = self.open(store, key)?.snapshot()?;
        if snapshot.project_id == key || snapshot.slug == key {
            return Ok(snapshot);
        }
        self.forget(key);
        let snapshot = self.open(store, key)?.snapshot()?;
        if snapshot.project_id == key || snapshot.slug == key {
            return Ok(snapshot);
        }
        self.forget(key);
        Err(StorageError::new(
            "PROJECT_NOT_FOUND",
            "Project does not exist",
        ))
    }

    /// Drops the pin for `key` and every other key holding the same project.
    fn forget(&self, key: &str) -> bool {
        let mut pins = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(project) = pins.remove(key) else {
            return false;
        };
        pins.retain(|_, pinned| !Arc::ptr_eq(pinned, &project));
        true
    }

    fn pin(&self, key: &str, project: &Arc<Project>) {
        let mut pins = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if pins.len() + 2 > MAX_KEYS && !pins.contains_key(key) {
            pins.clear();
        }
        for name in [key, project.id()] {
            match pins.get(name) {
                Some(pinned) if Arc::ptr_eq(pinned, project) => continue,
                // The slug now opens another project: the old handle's directory was replaced.
                Some(pinned) => {
                    let stale = pinned.clone();
                    pins.retain(|_, p| !Arc::ptr_eq(p, &stale));
                }
                None => {}
            }
            pins.insert(name.to_owned(), project.clone());
        }
    }
}
