//! Transactional internal schema migrations; authored scene schemas are separate.
use crate::*;

pub(crate) fn migrate(conn: &mut Connection) -> Result<()> {
    let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let meta = read_meta(conn)?;
    if version != meta.schema_version || !(1..=SCHEMA_VERSION).contains(&version) {
        return Err(StorageError::new(
            "SCHEMA_UNSUPPORTED",
            "Unsupported project database schema",
        ));
    }
    if version == SCHEMA_VERSION {
        return read_retention(conn).map(|_| ());
    }
    // All schema/data changes are transactional. Any failure rolls back both the
    // DDL and metadata. No asset I/O occurs while this transaction is active.
    let result = (|| -> Result<()> {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch("CREATE TABLE retention_metadata(singleton INTEGER PRIMARY KEY CHECK(singleton=1),oldest_commit_revision INTEGER NOT NULL CHECK(oldest_commit_revision>=1),idempotency_policy TEXT NOT NULL);
            INSERT INTO retention_metadata VALUES(1,1,'Retained for project lifetime; no expiration');
            UPDATE project_meta SET schema_version=2 WHERE singleton=1;
            PRAGMA user_version=2;")?;
        tx.commit()?;
        Ok(())
    })();
    result.map_err(|_| {
        StorageError::new(
            "MIGRATION_FAILED",
            "Project migration failed; original database retained",
        )
    })
}

pub(crate) fn read_retention(conn: &Connection) -> Result<Retention> {
    let (oldest, policy): (i64, String) = conn.query_row("SELECT oldest_commit_revision,idempotency_policy FROM retention_metadata WHERE singleton=1", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
    if oldest != 1 || policy != "Retained for project lifetime; no expiration" {
        return Err(StorageError::new(
            "PROJECT_INVALID",
            "Unsupported retention metadata",
        ));
    }
    Ok(Retention {
        oldest_commit_revision: oldest as u64,
        idempotency_policy: policy,
    })
}
