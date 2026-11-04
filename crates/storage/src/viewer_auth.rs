//! Workspace-wide public viewer credentials and revocable session records.
use crate::Result;
use serde::Serialize;

// Deliberately neither Debug nor Serialize. Credentials are only for the server.
pub struct ViewerAuthConfig {
    pub password_hash: String,
    pub signing_key: Vec<u8>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerSession {
    pub session_id: String,
    pub ip_address: String,
    pub user_agent: String,
    pub issued_at: i64,
    pub expires_at: i64,
}

pub trait ViewerAuthStore: Send + Sync {
    fn config(&self) -> Result<Option<ViewerAuthConfig>>;
    /// Replaces the credentials and revokes every session in one transaction.
    fn replace(&self, config: Option<ViewerAuthConfig>) -> Result<u64>;
    /// Inserts only while the hash used to verify the password is current.
    fn issue(&self, password_hash: String, session: ViewerSession, digest: Vec<u8>)
        -> Result<bool>;
    /// A valid, active token arriving from a different IP is revoked before returning false.
    fn check(&self, id: String, digest: Vec<u8>, ip: String, now: i64) -> Result<bool>;
    fn list(&self, now: i64) -> Result<Vec<ViewerSession>>;
    fn revoke(&self, id: String) -> Result<bool>;
}
