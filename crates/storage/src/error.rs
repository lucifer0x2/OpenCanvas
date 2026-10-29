use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Node detail of a property validation error, flattened into the error object.
    /// Boxed so the common error stays small.
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub node: Option<Box<NodeErrorContext>>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeErrorContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    /// Node whose authored properties failed validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_kind: Option<String>,
    /// Every property key that node kind accepts (see skill://opencanvas/props.md).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_keys: Option<Vec<String>>,
    /// The supported key an unsupported one most likely meant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}
pub type Result<T> = std::result::Result<T, StorageError>;
impl StorageError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
            current_revision: None,
            op_index: None,
            field: None,
            node: None,
        }
    }
    pub fn validation(field: &str, message: &str) -> Self {
        let mut e = Self::new("VALIDATION_FAILED", message);
        e.field = Some(field.into());
        e
    }
    pub fn conflict(revision: u64) -> Self {
        let mut e = Self::new(
            "REVISION_CONFLICT",
            "Read the current snapshot and retry with its revision",
        );
        e.current_revision = Some(revision);
        e
    }
    pub fn at(mut self, index: usize) -> Self {
        self.op_index = Some(index);
        self
    }
    /// Names the node a validation error belongs to, keeping any node already named.
    pub fn on_node(mut self, id: &str, kind: &str) -> Self {
        let node = self.node_context();
        node.node_id.get_or_insert_with(|| id.into());
        node.node_kind.get_or_insert_with(|| kind.into());
        self
    }
    pub fn node_context(&mut self) -> &mut NodeErrorContext {
        self.node.get_or_insert_default()
    }
}
impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for StorageError {}
impl From<std::io::Error> for StorageError {
    fn from(_: std::io::Error) -> Self {
        Self::new("STORAGE_IO", "Project storage operation failed")
    }
}
impl From<serde_json::Error> for StorageError {
    fn from(_: serde_json::Error) -> Self {
        Self::validation("json", "Invalid or unsupported JSON data")
    }
}
impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        if matches!(
            e.sqlite_error_code(),
            Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
        ) {
            let mut err = Self::new(
                "STORAGE_BUSY",
                "Project storage is busy; retry after a short delay",
            );
            err.retryable = true;
            err
        } else {
            Self::new("STORAGE_FAILED", "Project database operation failed")
        }
    }
}
