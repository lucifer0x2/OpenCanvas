use super::SqliteBackend;
use crate::{
    no_symlink, poisoned, sync_dir, Connection, OpenFlags, OpenOptions, OptionalExtension, Result,
    TransactionBehavior, ViewerAuthConfig, ViewerAuthStore, ViewerSession,
};
use rusqlite::{params, Row};
use std::{os::unix::fs::OpenOptionsExt, time::Duration};

const FILE: &str = ".public-viewer.sqlite";
const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS viewer_credentials (
        singleton INTEGER PRIMARY KEY CHECK(singleton=1), password_hash TEXT, signing_key BLOB);
    CREATE TABLE IF NOT EXISTS viewer_sessions (
        session_id TEXT PRIMARY KEY, token_digest BLOB NOT NULL, ip_address TEXT NOT NULL,
        user_agent TEXT NOT NULL, issued_at INTEGER NOT NULL, expires_at INTEGER NOT NULL);";

impl SqliteBackend {
    fn viewer_db<T>(
        &self,
        create: bool,
        missing: T,
        operation: impl FnOnce(&mut Connection) -> Result<T>,
    ) -> Result<T> {
        let root = &self.store.root;
        let mut slot = root.viewer_db.lock().map_err(|_| poisoned())?;
        if slot.is_none() {
            root.check_path()?;
            let path = root.path.join(FILE);
            no_symlink(&path)?;
            if !path.try_exists()? {
                if !create {
                    return Ok(missing);
                }
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&path)?;
            }
            no_symlink(&root.path.join(format!("{FILE}-journal")))?;
            let connection = Connection::open_with_flags(
                &path,
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
            )?;
            connection.busy_timeout(Duration::from_secs(3))?;
            connection.pragma_update(None, "synchronous", "FULL")?;
            connection.pragma_update(None, "fullfsync", "ON")?;
            connection.execute_batch(SCHEMA)?;
            sync_dir(&root.path)?;
            *slot = Some(connection);
        }
        operation(slot.as_mut().expect("viewer database opened"))
    }
}

impl ViewerAuthStore for SqliteBackend {
    fn config(&self) -> Result<Option<ViewerAuthConfig>> {
        self.viewer_db(false, None, |db| {
            Ok(db.query_row(
                "SELECT password_hash, signing_key FROM viewer_credentials WHERE password_hash IS NOT NULL",
                [],
                |row| Ok(ViewerAuthConfig { password_hash: row.get(0)?, signing_key: row.get(1)? }),
            ).optional()?)
        })
    }

    fn replace(&self, config: Option<ViewerAuthConfig>) -> Result<u64> {
        self.viewer_db(config.is_some(), 0, |db| {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let (hash, key) = match config {
                Some(config) => (Some(config.password_hash), Some(config.signing_key)),
                None => (None, None),
            };
            tx.execute(
                "INSERT INTO viewer_credentials VALUES (1, ?1, ?2) ON CONFLICT(singleton)
                 DO UPDATE SET password_hash=excluded.password_hash, signing_key=excluded.signing_key",
                params![hash, key],
            )?;
            let revoked = tx.execute("DELETE FROM viewer_sessions", [])?;
            tx.commit()?;
            Ok(revoked as u64)
        })
    }

    fn issue(&self, hash: String, session: ViewerSession, digest: Vec<u8>) -> Result<bool> {
        self.viewer_db(false, false, |db| {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let inserted = tx.execute(
                "INSERT INTO viewer_sessions (session_id, token_digest, ip_address, user_agent, issued_at, expires_at)
                 SELECT ?1, ?2, ?3, ?4, ?5, ?6 FROM viewer_credentials WHERE password_hash=?7",
                params![session.session_id, digest, session.ip_address, session.user_agent,
                    session.issued_at, session.expires_at, hash],
            )?;
            if inserted > 0 {
                tx.execute("DELETE FROM viewer_sessions WHERE expires_at<=?1", [session.issued_at])?;
            }
            tx.commit()?;
            Ok(inserted > 0)
        })
    }

    fn check(&self, id: String, digest: Vec<u8>, ip: String, now: i64) -> Result<bool> {
        self.viewer_db(false, false, |db| {
            let recorded: Option<String> = db.query_row(
                "SELECT ip_address FROM viewer_sessions WHERE session_id=?1 AND token_digest=?2 AND expires_at>?3",
                params![id, digest, now],
                |row| row.get(0),
            ).optional()?;
            if recorded.as_ref().is_some_and(|saved| saved != &ip) {
                db.execute("DELETE FROM viewer_sessions WHERE session_id=?1 AND token_digest=?2", params![id, digest])?;
            }
            Ok(recorded.as_ref() == Some(&ip))
        })
    }

    fn list(&self, now: i64) -> Result<Vec<ViewerSession>> {
        self.viewer_db(false, vec![], |db| {
            let mut stmt = db.prepare(
                "SELECT session_id, ip_address, user_agent, issued_at, expires_at FROM viewer_sessions
                 WHERE expires_at>?1 ORDER BY issued_at DESC, session_id",
            )?;
            let rows = stmt.query_map([now], decode)?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }

    fn revoke(&self, id: String) -> Result<bool> {
        self.viewer_db(false, false, |db| {
            Ok(db.execute("DELETE FROM viewer_sessions WHERE session_id=?1", [id])? > 0)
        })
    }
}

fn decode(row: &Row<'_>) -> rusqlite::Result<ViewerSession> {
    Ok(ViewerSession {
        session_id: row.get(0)?,
        ip_address: row.get(1)?,
        user_agent: row.get(2)?,
        issued_at: row.get(3)?,
        expires_at: row.get(4)?,
    })
}
