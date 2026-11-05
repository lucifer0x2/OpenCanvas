use super::{now, LIFETIME};
use crate::{error::Result, ApiError};
use argon2::{
    password_hash::{Error, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use opencanvas_storage::ViewerAuthConfig;
use ring::{
    hmac,
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const HEADER: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"; // {"alg":"HS256","typ":"JWT"}

fn failed() -> ApiError {
    ApiError::new("INTERNAL_ERROR", "Viewer authentication operation failed")
}

pub(super) fn credentials(password: &str) -> Result<ViewerAuthConfig> {
    let random = SystemRandom::new();
    let mut salt_bytes = [0; 16];
    let mut signing_key = vec![0; 32];
    random.fill(&mut salt_bytes).map_err(|_| failed())?;
    random.fill(&mut signing_key).map_err(|_| failed())?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| failed())?;
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| failed())?
        .to_string();
    Ok(ViewerAuthConfig {
        password_hash,
        signing_key,
    })
}

pub(super) fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let hash = PasswordHash::new(hash).map_err(|_| failed())?;
    match Argon2::default().verify_password(password.as_bytes(), &hash) {
        Ok(()) => Ok(true),
        Err(Error::Password) => Ok(false),
        Err(_) => Err(failed()),
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Claims {
    pub jti: Uuid,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
}

pub(super) fn sign(origin: &str, key: &[u8]) -> Result<(String, Claims)> {
    let issued = now();
    let claims = Claims {
        jti: Uuid::new_v4(),
        aud: origin.into(),
        iat: issued,
        exp: issued + LIFETIME,
    };
    let payload = B64.encode(serde_json::to_vec(&claims).map_err(|_| failed())?);
    let message = format!("{HEADER}.{payload}");
    let key = hmac::Key::new(hmac::HMAC_SHA256, key);
    let tag = B64.encode(hmac::sign(&key, message.as_bytes()).as_ref());
    Ok((format!("{message}.{tag}"), claims))
}

pub(super) fn verify(token: &str, origin: &str, key: &[u8]) -> Option<Claims> {
    if token.len() > 2048 {
        return None;
    }
    let (message, tag) = token.rsplit_once('.')?;
    let payload = message.strip_prefix(HEADER)?.strip_prefix('.')?;
    let key = hmac::Key::new(hmac::HMAC_SHA256, key);
    hmac::verify(&key, message.as_bytes(), &B64.decode(tag).ok()?).ok()?;
    let claims: Claims = serde_json::from_slice(&B64.decode(payload).ok()?).ok()?;
    let now = now();
    (claims.aud == origin && claims.iat <= now && now < claims.exp).then_some(claims)
}

pub(super) fn digest(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}
