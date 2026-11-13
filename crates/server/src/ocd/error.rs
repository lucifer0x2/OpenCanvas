//! Where a source failed to parse: 1-based line and character column, a message and, for an
//! unknown key, the key the author most likely meant.
use crate::error::ApiError;
use opencanvas_storage::StorageError;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcdError {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub suggestion: Option<String>,
}

pub type Result<T> = std::result::Result<T, OcdError>;

impl OcdError {
    pub fn new(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            column,
            message: message.into(),
            suggestion: None,
        }
    }
    pub fn suggest(mut self, suggestion: Option<&str>) -> Self {
        self.suggestion = suggestion.map(Into::into);
        self
    }
}

/// `SOURCE_PARSE_FAILED` with `field:"source"`, `line`, `column` and, when known, the same
/// top-level `suggestion` field that property validation errors carry.
impl From<OcdError> for ApiError {
    fn from(e: OcdError) -> Self {
        let mut error = StorageError::new("SOURCE_PARSE_FAILED", &e.message);
        error.field = Some("source".into());
        if let Some(suggestion) = e.suggestion {
            error.node_context().suggestion = Some(suggestion);
        }
        ApiError::from(error)
            .with_detail("line", json!(e.line))
            .with_detail("column", json!(e.column))
    }
}
