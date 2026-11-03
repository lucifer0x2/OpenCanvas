//! Bounded strong handles keep SQLite's Weak registry hot without a Root/Project cycle.
use crate::{Project, ProjectMeta, Result, StorageError, Store};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

/// One ID and at most one slug per project. Clearing bounds live SQLite writers;
/// projects reopen lazily. The adapter, rather than Root, owns these references.
const MAX_KEYS: usize = 128;

#[derive(Default)]
pub(super) struct ProjectPins(Mutex<HashMap<String, Arc<Project>>>);

impl ProjectPins {
    /// Read just identity metadata before handing a writer to a domain operation.
    /// A replaced directory must never pair new reader data with the old writer.
    pub fn open(&self, store: &Store, key: &str) -> Result<Arc<Project>> {
        self.read(store, key, |project| {
            let meta = crate::read_meta(&project.reader()?)?;
            Ok((meta, ()))
        })
        .map(|(project, ())| project)
    }

    pub fn metadata(&self, store: &Store, key: &str) -> Result<ProjectMeta> {
        self.read(store, key, |project| {
            let meta = crate::read_meta(&project.reader()?)?;
            Ok((meta.clone(), meta))
        })
        .map(|(_, meta)| meta)
    }

    /// Resolve and inspect in one read; snapshot callers avoid a second connection
    /// and metadata query. Both the lookup alias and cached writer ID must match.
    pub fn read<T>(
        &self,
        store: &Store,
        key: &str,
        read: impl Fn(&Project) -> Result<(ProjectMeta, T)>,
    ) -> Result<(Arc<Project>, T)> {
        for attempt in 0..2 {
            let result = store.open_project(key).and_then(|project| {
                let (meta, value) = read(&project)?;
                if meta.project_id != project.id() || (meta.project_id != key && meta.slug != key) {
                    return Err(StorageError::new(
                        "PROJECT_NOT_FOUND",
                        "Project does not exist",
                    ));
                }
                Ok((project, value))
            });
            match result {
                Ok((project, value)) => {
                    self.pin(key, &project);
                    return Ok((project, value));
                }
                Err(_) if attempt == 0 && self.forget(store, key) => continue,
                Err(error) => return Err(error),
            }
        }
        unreachable!("The final attempt always returns")
    }

    fn forget(&self, store: &Store, key: &str) -> bool {
        let mut pins = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        // Legacy Store callers may still hold a strong reference. Remove matching
        // Weak aliases too, so that reference cannot revive a stale directory.
        let mut registry = store
            .root
            .projects
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let project = pins
            .remove(key)
            .or_else(|| registry.get(key).and_then(std::sync::Weak::upgrade));
        let Some(project) = project else {
            return false;
        };
        pins.retain(|_, pinned| !Arc::ptr_eq(pinned, &project));
        registry.retain(|_, weak| !weak.ptr_eq(&Arc::downgrade(&project)));
        true
    }

    pub fn pin(&self, key: &str, project: &Arc<Project>) {
        let mut pins = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if pins.len() + 2 > MAX_KEYS && !pins.contains_key(key) {
            pins.clear();
        }
        for name in [key, project.id()] {
            match pins.get(name) {
                Some(pinned) if Arc::ptr_eq(pinned, project) => continue,
                Some(pinned) => {
                    let stale = pinned.clone();
                    pins.retain(|_, pinned| !Arc::ptr_eq(pinned, &stale));
                }
                None => {}
            }
            pins.insert(name.to_owned(), project.clone());
        }
    }
}

#[cfg(test)]
mod tests;
