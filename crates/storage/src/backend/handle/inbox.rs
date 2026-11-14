//! Asset imports may name a file in the project's inbox instead of carrying its bytes.
//! The inbox is a local directory for every engine: SQLite uses its data root, and any
//! other backend uses the root given to `with_inbox_root`. Nothing here creates a path.
use super::{ProjectHandle, Storage};
use crate::{
    inbox::{self, Purpose},
    validate::validate_slug,
    Result, StorageError,
};
use std::{path::PathBuf, sync::Arc};

impl Storage {
    /// Selects the local directory whose `<slug>/inbox` holds files for path imports.
    /// It is resolved on each read, so it may appear after startup.
    pub fn with_inbox_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.inbox_root = Some(Arc::new(root.into()));
        self
    }
}

impl ProjectHandle {
    /// Bytes of one asset file in this project's inbox, at most `MAX_ASSET_BYTES`, under
    /// the same containment rules as staged batches: no traversal, symlinks or special files.
    pub fn read_inbox_asset(&self, path: &str) -> Result<Vec<u8>> {
        let meta = self.storage.backend.project_meta(self.id())?;
        validate_slug(&meta.slug)?;
        let root = self.storage.inbox_root.as_deref().ok_or_else(|| {
            StorageError::new(
                "STAGING_UNSUPPORTED",
                "This server has no filesystem inbox; send base64",
            )
        })?;
        // The descriptor walk opens the canonical root; an absent root is an absent file.
        let root = std::fs::canonicalize(root).map_err(|_| Purpose::Asset.missing())?;
        inbox::read(&root, &root.join(&meta.slug), path, Purpose::Asset)
    }
}
