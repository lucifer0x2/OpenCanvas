//! A completed SQLite backup, never a copy of a live main database file.
use crate::*;

pub(crate) const MAX_DB_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) fn capture(project: &Project, destination: &Path) -> Result<Snapshot> {
    project.check_paths()?;
    let source = project.reader()?;
    let mut target = Connection::open(destination)?;
    // One step copies one consistent database image. The independent reader may
    // pin WAL briefly; it never owns the project writer mutex or asset I/O.
    let backup = rusqlite::backup::Backup::new(&source, &mut target)?;
    match backup.step(-1)? {
        rusqlite::backup::StepResult::Done => (),
        _ => {
            let mut error = StorageError::new("STORAGE_BUSY", "Backup source is busy; retry");
            error.retryable = true;
            return Err(error);
        }
    }
    drop(backup);
    // Backup may copy WAL journal-mode metadata from the source. Collapse any
    // target WAL before measuring/serializing the standalone database file.
    target.pragma_update(None, "journal_mode", "DELETE")?;
    if fs::metadata(destination)?.len() > MAX_DB_BYTES {
        return Err(StorageError::new(
            "BUNDLE_LIMIT",
            "Database exceeds bundle size limit",
        ));
    }
    let snapshot = read_snapshot(&mut target)?;
    target.close().map_err(|(_, e)| StorageError::from(e))?;
    File::open(destination)?.sync_all()?;
    Ok(snapshot)
}
