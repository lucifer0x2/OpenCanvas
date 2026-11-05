use crate::{error::Result, AppState};
use opencanvas_storage::{ViewerAuthConfig, ViewerAuthStore};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) mod admin;
mod crypto;
pub(crate) mod http;
pub(crate) mod limits;
pub(crate) mod stream;

pub(crate) const PATH: &str = "/api/v1/viewer/auth";
const MAX_PASSWORD_CHARS: usize = 1024;
const LIFETIME: i64 = 90 * 24 * 60 * 60;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

async fn config(state: &AppState) -> Result<Option<ViewerAuthConfig>> {
    state
        .db(|store| {
            store
                .viewer_auth()
                .map(ViewerAuthStore::config)
                .transpose()
                .map(Option::flatten)
        })
        .await
}

fn valid_password(password: &str) -> bool {
    !password.is_empty() && password.chars().count() <= MAX_PASSWORD_CHARS
}
