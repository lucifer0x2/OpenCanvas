use crate::*;

/// Only enabled explicitly by recovery tests; production builds cannot be
/// terminated by this environment variable. exit() bypasses Rust destructors.
pub(crate) fn failpoint(name: &str) {
    #[cfg(feature = "fault-injection")]
    if std::env::var("OPENCANVAS_STORAGE_FAILPOINT").as_deref() == Ok(name) {
        std::process::exit(91);
    }
    let _ = name;
}
pub(crate) fn poisoned() -> StorageError {
    StorageError::new("STORAGE_FAILED", "Project storage state is unavailable")
}
pub(crate) fn us(t: Instant) -> u64 {
    t.elapsed().as_micros().min(u64::MAX as u128) as u64
}
pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn sync_dir(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}
pub(crate) fn no_symlink(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => Err(StorageError::new(
            "PATH_REJECTED",
            "Symlinks are not allowed in project storage",
        )),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
pub(crate) fn guard(path: &Path, parent: &Path) -> Result<PathBuf> {
    no_symlink(path)?;
    let real = fs::canonicalize(path)?;
    if !real.starts_with(parent) || real == parent {
        return Err(StorageError::new(
            "PATH_REJECTED",
            "Project path escapes its storage root",
        ));
    }
    Ok(real)
}
pub(crate) fn version_check() -> Result<()> {
    if rusqlite::version_number() < 3_051_003 {
        return Err(StorageError::new(
            "SQLITE_VERSION_UNSUPPORTED",
            "Bundled SQLite 3.51.3 or newer is required",
        ));
    }
    Ok(())
}
pub(crate) fn configure(conn: &Connection, writer: bool) -> Result<()> {
    version_check()?;
    conn.busy_timeout(Duration::from_millis(1500))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    if writer {
        let mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
        if mode.to_lowercase() != "wal" {
            return Err(StorageError::new(
                "STORAGE_FAILED",
                "WAL mode is unavailable",
            ));
        }
        conn.pragma_update(None, "wal_autocheckpoint", 1000)?;
        conn.pragma_update(None, "journal_size_limit", 16 * 1024 * 1024)?;
        // On macOS, request F_FULLFSYNC in addition to SQLite FULL durability.
        conn.pragma_update(None, "fullfsync", "ON")?;
        conn.pragma_update(None, "checkpoint_fullfsync", "ON")?;
    }
    Ok(())
}
