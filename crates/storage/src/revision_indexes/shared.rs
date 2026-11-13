//! Explicit authority binding: identity metadata alone never grants read trust.
use crate::*;
use opencanvas_scene_input::read_scope::{ScopeIndex, ScopedSnapshot};

pub struct SharedSnapshot {
    pub snapshot: Arc<Snapshot>,
    pub indexes: Arc<RevisionIndexes>,
    authority: Option<(Arc<Snapshot>, Arc<RevisionIndexes>)>,
}

impl SharedSnapshot {
    /// Custom/embedded backends remain conservative, even with a cache identity.
    pub fn untrusted(snapshot: Arc<Snapshot>, indexes: Arc<RevisionIndexes>) -> Self {
        Self {
            snapshot,
            indexes,
            authority: None,
        }
    }

    /// Builtin storage owns the validated-write invariant. Bound to the exact
    /// immutable pair; callers replacing either public Arc lose this authority.
    pub(crate) fn committed(snapshot: Arc<Snapshot>, indexes: Arc<RevisionIndexes>) -> Self {
        Self {
            authority: Some((snapshot.clone(), indexes.clone())),
            snapshot,
            indexes,
        }
    }

    pub fn scoped(&self) -> Result<ScopedSnapshot<'_>> {
        if let Some(scoped) = self.trusted_scoped()? {
            return Ok(scoped);
        }
        ScopedSnapshot::validate(&self.snapshot)
    }

    /// Read layers with a legacy unvalidated fallback (for example find) can
    /// preserve it for custom backends rather than introducing new validation.
    pub fn trusted_scoped(&self) -> Result<Option<ScopedSnapshot<'_>>> {
        if !self.authority.as_ref().is_some_and(|(snapshot, indexes)| {
            Arc::ptr_eq(snapshot, &self.snapshot) && Arc::ptr_eq(indexes, &self.indexes)
        }) {
            return Ok(None);
        }
        let mut cached = self.indexes.read_scope.lock().map_err(|_| poisoned())?;
        // Built on the first authoritative *snapshot vector*, not Candidate's
        // BTreeMap order. Commits publish indexes with this lazy slot empty.
        let index = if let Some(index) = cached.as_ref() {
            index.clone()
        } else {
            bench_phase::count("reads.scope_index_builds", 1);
            let index = Arc::new(ScopeIndex::new(&self.snapshot));
            // Benchmark-only oversized note graphs are usable but not retained.
            if index.estimated_bytes() <= self.indexes.read_weight {
                *cached = Some(index.clone());
            }
            index
        };
        Ok(Some(ScopedSnapshot::from_validated(&self.snapshot, index)))
    }

    pub fn edit_context(&self, pages: Vec<String>) -> Result<Value> {
        crate::scoped::edit_context_indexed(&self.snapshot, &self.indexes, pages)
    }
}
