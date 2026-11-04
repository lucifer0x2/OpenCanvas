//! Transactional internal schema migrations; authored scene schemas are separate.
use crate::*;

// SQLite's storage version advances separately from the authored snapshot schema.
pub(crate) const DATABASE_SCHEMA_VERSION: u32 = 5;

pub(crate) fn database_version(conn: &Connection) -> Result<u32> {
    let version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let meta_version: u32 = conn.query_row(
        "SELECT schema_version FROM project_meta WHERE singleton=1",
        [],
        |row| row.get(0),
    )?;
    if version != meta_version || !(1..=DATABASE_SCHEMA_VERSION).contains(&version) {
        return Err(StorageError::new(
            "SCHEMA_UNSUPPORTED",
            "Unsupported project database schema",
        ));
    }
    Ok(version)
}

pub(crate) fn migrate(conn: &mut Connection) -> Result<()> {
    migrate_to(conn, DATABASE_SCHEMA_VERSION)
}

pub(crate) fn migrate_to(conn: &mut Connection, target_version: u32) -> Result<()> {
    let version = database_version(conn)?;
    if version == target_version && version >= 2 {
        return read_retention(conn).map(|_| ());
    }
    if version == target_version {
        return Ok(());
    }
    // Every step runs in one transaction. Any failure rolls back all DDL and the
    // metadata. No asset I/O occurs while this transaction is active.
    let result = (|| -> Result<()> {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if version < 2 && target_version >= 2 {
            tx.execute_batch("CREATE TABLE retention_metadata(singleton INTEGER PRIMARY KEY CHECK(singleton=1),oldest_commit_revision INTEGER NOT NULL CHECK(oldest_commit_revision>=1),idempotency_policy TEXT NOT NULL);
                INSERT INTO retention_metadata VALUES(1,1,'Retained for project lifetime; no expiration');")?;
        }
        if version < 3 && target_version >= 3 {
            tx.execute_batch("CREATE TABLE notes(id TEXT PRIMARY KEY,data_json TEXT NOT NULL);")?;
        }
        if version < 5 && target_version >= 5 {
            tx.execute_batch(
                "ALTER TABLE project_meta ADD COLUMN privacy_status TEXT NOT NULL DEFAULT 'private'
                 CHECK(privacy_status IN ('private','public'));",
            )?;
        }
        tx.execute_batch(&format!(
            "UPDATE project_meta SET schema_version={target_version} WHERE singleton=1;
            PRAGMA user_version={target_version};"
        ))?;
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
