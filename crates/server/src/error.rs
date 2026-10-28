use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use opencanvas_storage::StorageError;
use serde_json::{json, Map, Value};

/// A storage-shaped error plus optional server-added detail fields (such as `changedSince` on
/// `REVISION_CONFLICT`). Details are merged into the `error` object; they never replace the
/// storage fields, so `code`, `message`, `retryable` and `currentRevision` stay as storage set them.
#[derive(Debug, Clone)]
pub struct ApiError(pub StorageError, Option<Box<Map<String, Value>>>);
pub type Result<T> = std::result::Result<T, ApiError>;
impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        let mut error = StorageError::new(code, message);
        error.retryable = matches!(
            code,
            "QUEUE_FULL" | "JOB_TIMEOUT" | "WORKER_UNAVAILABLE" | "WORKER_FAILED"
        );
        Self(error, None)
    }
    pub fn invalid(message: &str) -> Self {
        Self::new("INVALID_REQUEST", message)
    }
    /// Adds a detail field to the serialized `error` object; storage fields win on a name clash.
    pub fn with_detail(mut self, key: &str, value: Value) -> Self {
        self.1.get_or_insert_default().insert(key.into(), value);
        self
    }
    pub fn value(&self) -> Value {
        let mut error = json!(self.0);
        if let (Some(details), Some(object)) = (&self.1, error.as_object_mut()) {
            for (key, value) in details.iter() {
                object.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }
        json!({ "error": error })
    }
}
impl From<StorageError> for ApiError {
    fn from(e: StorageError) -> Self {
        Self(e, None)
    }
}
impl From<serde_json::Error> for ApiError {
    fn from(_: serde_json::Error) -> Self {
        Self::invalid("Invalid request fields or types")
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0.code.as_str() {
            "REVISION_CONFLICT" | "IDEMPOTENCY_KEY_REUSED" | "REVISION_UNAVAILABLE" => {
                StatusCode::CONFLICT
            }
            "PROJECT_NOT_FOUND" | "ASSET_NOT_FOUND" | "JOB_NOT_FOUND" => StatusCode::NOT_FOUND,
            "QUEUE_FULL" | "STORAGE_BUSY" => StatusCode::SERVICE_UNAVAILABLE,
            "INTERNAL_ERROR" | "STORAGE_FAILED" | "STORAGE_IO" => StatusCode::INTERNAL_SERVER_ERROR,
            "IMAGE_NOT_READY" => StatusCode::CONFLICT,
            _ => StatusCode::BAD_REQUEST,
        };
        (status, Json(self.value())).into_response()
    }
}
