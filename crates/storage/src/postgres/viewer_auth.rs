use super::{sql, PostgresBackend};
use crate::{Result, ViewerAuthConfig, ViewerAuthStore, ViewerSession};

pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS viewer_credentials (
        singleton BOOLEAN PRIMARY KEY CHECK(singleton), password_hash TEXT, signing_key BYTEA);
    CREATE TABLE IF NOT EXISTS viewer_sessions (
        session_id TEXT PRIMARY KEY, token_digest BYTEA NOT NULL, ip_address TEXT NOT NULL,
        user_agent TEXT NOT NULL, issued_at BIGINT NOT NULL, expires_at BIGINT NOT NULL);";

impl ViewerAuthStore for PostgresBackend {
    fn config(&self) -> Result<Option<ViewerAuthConfig>> {
        self.pool.run(|session| {
            Ok(session.db().query_opt(
                "SELECT password_hash, signing_key FROM viewer_credentials WHERE password_hash IS NOT NULL",
                &[],
            )?.map(|row| ViewerAuthConfig { password_hash: row.get(0), signing_key: row.get(1) }))
        })
    }

    fn replace(&self, config: Option<ViewerAuthConfig>) -> Result<u64> {
        self.pool.run(move |session| {
            let mut tx = session.client.transaction().map_err(sql)?;
            let (hash, key) = match config {
                Some(config) => (Some(config.password_hash), Some(config.signing_key)),
                None => (None, None),
            };
            tx.execute(
                "INSERT INTO viewer_credentials VALUES (true, $1, $2) ON CONFLICT(singleton)
                 DO UPDATE SET password_hash=excluded.password_hash, signing_key=excluded.signing_key",
                &[&hash, &key],
            ).map_err(sql)?;
            let revoked = tx.execute("DELETE FROM viewer_sessions", &[]).map_err(sql)?;
            tx.commit().map_err(sql)?;
            Ok(revoked)
        })
    }

    fn issue(&self, hash: String, viewer: ViewerSession, digest: Vec<u8>) -> Result<bool> {
        self.pool.run(move |session| {
            let mut tx = session.client.transaction().map_err(sql)?;
            // The credential row orders issuance against change and disable.
            let current = tx.query_opt(
                "SELECT singleton FROM viewer_credentials WHERE password_hash=$1 FOR UPDATE",
                &[&hash],
            ).map_err(sql)?;
            if current.is_none() {
                return Ok(false);
            }
            tx.execute(
                "INSERT INTO viewer_sessions (session_id, token_digest, ip_address, user_agent, issued_at, expires_at)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[&viewer.session_id, &digest, &viewer.ip_address, &viewer.user_agent,
                  &viewer.issued_at, &viewer.expires_at],
            ).map_err(sql)?;
            tx.execute("DELETE FROM viewer_sessions WHERE expires_at<=$1", &[&viewer.issued_at]).map_err(sql)?;
            tx.commit().map_err(sql)?;
            Ok(true)
        })
    }

    fn check(&self, id: String, digest: Vec<u8>, ip: String, now: i64) -> Result<bool> {
        self.pool.run(move |session| {
            // Delete a mismatched active bearer atomically, without writing on a matching read.
            let result = session.db().query_opt(
                "WITH revoked AS (DELETE FROM viewer_sessions
                    WHERE session_id=$1 AND token_digest=$2 AND expires_at>$3 AND ip_address<>$4)
                 SELECT session_id FROM viewer_sessions
                 WHERE session_id=$1 AND token_digest=$2 AND expires_at>$3 AND ip_address=$4",
                &[&id, &digest, &now, &ip],
            )?;
            Ok(result.is_some())
        })
    }

    fn list(&self, now: i64) -> Result<Vec<ViewerSession>> {
        self.pool.run(move |session| {
            Ok(session.db().query(
                "SELECT session_id, ip_address, user_agent, issued_at, expires_at FROM viewer_sessions
                 WHERE expires_at>$1 ORDER BY issued_at DESC, session_id",
                &[&now],
            )?.into_iter().map(|row| ViewerSession {
                session_id: row.get(0), ip_address: row.get(1), user_agent: row.get(2),
                issued_at: row.get(3), expires_at: row.get(4),
            }).collect())
        })
    }

    fn revoke(&self, id: String) -> Result<bool> {
        self.pool.run(move |session| {
            Ok(session
                .db()
                .execute("DELETE FROM viewer_sessions WHERE session_id=$1", &[&id])?
                > 0)
        })
    }
}
