use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use opencanvas_storage::StorageError;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct ApiError(pub StorageError);
pub type Result<T> = std::result::Result<T, ApiError>;
impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        let mut error = StorageError::new(code, message);
        error.retryable = matches!(
            code,
            "QUEUE_FULL" | "JOB_TIMEOUT" | "WORKER_UNAVAILABLE" | "WORKER_FAILED"
        );
        Self(error)
    }
    pub fn invalid(message: &str) -> Self {
        Self::new("INVALID_REQUEST", message)
    }
    pub fn value(&self) -> serde_json::Value {
        json!({"error":self.0})
    }
}
impl From<StorageError> for ApiError {
    fn from(e: StorageError) -> Self {
        Self(e)
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
