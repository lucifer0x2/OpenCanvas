use super::{crypto, now, valid_password, MAX_PASSWORD_CHARS};
use crate::{error::Result, schemas::object, ApiError, AppState};
use opencanvas_storage::{Storage, StorageError, ViewerAuthStore};
use serde_json::{json, Value};
use uuid::Uuid;

const SET: &str = "set_public_viewer_password";
const LIST: &str = "list_public_viewer_sessions";
const REVOKE: &str = "revoke_public_viewer_session";

pub(crate) fn tools(mut add: impl FnMut(&'static str, String, Value)) {
    add(
        SET,
        "Set or change the workspace public viewer password. Pass null to disable. Revokes all existing viewer sessions.".into(),
        object(json!({"password":{"type":["string","null"],"minLength":1,"maxLength":MAX_PASSWORD_CHARS}}), &["password"]),
    );
    add(
        LIST,
        "List active public viewer sessions with client IP, user agent and Unix timestamps.".into(),
        object(json!({}), &[]),
    );
    add(
        REVOKE,
        "Revoke one public viewer session by its listed sessionId.".into(),
        object(
            json!({"sessionId":{"type":"string","minLength":36,"maxLength":36}}),
            &["sessionId"],
        ),
    );
}

pub(crate) fn is_tool(name: &str) -> bool {
    matches!(name, SET | LIST | REVOKE)
}

pub(crate) async fn call(state: &AppState, name: &str, args: Value) -> Result<Value> {
    match name {
        SET => {
            let credentials = match args["password"].as_str() {
                Some(password) if valid_password(password) => {
                    let password = password.to_owned();
                    let permit = state.viewer_auth_limits.try_admin_hash().ok_or_else(|| {
                        ApiError::new("QUEUE_FULL", "Viewer authentication is busy")
                    })?;
                    Some(
                        tokio::task::spawn_blocking(move || {
                            let _permit = permit;
                            crypto::credentials(&password)
                        })
                        .await
                        .map_err(|_| {
                            ApiError::new("INTERNAL_ERROR", "Password hashing failed")
                        })??,
                    )
                }
                Some(_) => return Err(ApiError::invalid("Password length is invalid")),
                None => None,
            };
            let required = credentials.is_some();
            let revoked = state
                .db(move |store| auth(&store)?.replace(credentials))
                .await?;
            Ok(json!({"required":required,"revokedSessions":revoked}))
        }
        LIST => {
            let sessions = state.db(move |store| auth(&store)?.list(now())).await?;
            Ok(json!({"sessions":sessions}))
        }
        REVOKE => {
            let id = args["sessionId"]
                .as_str()
                .and_then(|id| id.parse::<Uuid>().ok())
                .ok_or_else(|| ApiError::invalid("Invalid viewer sessionId"))?;
            let revoked = state
                .db(move |store| auth(&store)?.revoke(id.to_string()))
                .await?;
            Ok(json!({"revoked":revoked}))
        }
        _ => Err(ApiError::new("UNKNOWN_TOOL", "Tool is not available")),
    }
}

pub(super) fn auth(store: &Storage) -> opencanvas_storage::Result<&dyn ViewerAuthStore> {
    store.viewer_auth().ok_or_else(|| {
        StorageError::new(
            "VIEWER_AUTH_UNSUPPORTED",
            "This storage backend does not support viewer authentication",
        )
    })
}
