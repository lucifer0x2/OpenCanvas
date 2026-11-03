//! Client-safe classification of PostgreSQL driver failures.
//! Messages are fixed strings: no URL, host, credential or driver text is echoed.
use ::postgres::error::SqlState;
use std::io::ErrorKind;

/// Retry class of a driver failure, decided without a database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Failure {
    /// Contention inside a healthy server: retry the same operation.
    Busy,
    /// The connection or server is gone or refusing work: reconnect and retry.
    Unavailable,
    /// Everything else, including authentication and unknown databases.
    Failed,
}

pub(super) fn unavailable() -> crate::StorageError {
    let mut error = crate::StorageError::new(
        "STORAGE_UNAVAILABLE",
        "Project database is unavailable; retry after a short delay",
    );
    error.retryable = true;
    error
}

fn busy() -> crate::StorageError {
    let mut error = crate::StorageError::new(
        "STORAGE_BUSY",
        "Project storage is busy; retry after a short delay",
    );
    error.retryable = true;
    error
}

pub(super) fn sql(error: ::postgres::Error) -> crate::StorageError {
    let io = std::error::Error::source(&error)
        .and_then(|cause| cause.downcast_ref::<std::io::Error>())
        .map(std::io::Error::kind);
    match classify(error.code(), error.is_closed(), io) {
        Failure::Busy => busy(),
        Failure::Unavailable => unavailable(),
        Failure::Failed => {
            crate::StorageError::new("STORAGE_FAILED", "Project database operation failed")
        }
    }
}

/// A SQLSTATE wins; otherwise only a closed connection or a transport I/O failure
/// is retryable. Malformed input or refused permission from the driver is not.
pub(super) fn classify(code: Option<&SqlState>, closed: bool, io: Option<ErrorKind>) -> Failure {
    match code {
        Some(code) if busy_state(code) => Failure::Busy,
        Some(code) if unavailable_state(code) => Failure::Unavailable,
        Some(_) => Failure::Failed,
        None if closed || io.is_some_and(transport) => Failure::Unavailable,
        None => Failure::Failed,
    }
}

fn busy_state(code: &SqlState) -> bool {
    [
        SqlState::LOCK_NOT_AVAILABLE,
        SqlState::QUERY_CANCELED,
        SqlState::T_R_DEADLOCK_DETECTED,
    ]
    .contains(code)
}

fn unavailable_state(code: &SqlState) -> bool {
    [
        SqlState::ADMIN_SHUTDOWN,
        SqlState::CRASH_SHUTDOWN,
        SqlState::CANNOT_CONNECT_NOW,
        SqlState::TOO_MANY_CONNECTIONS,
        SqlState::CONNECTION_EXCEPTION,
        SqlState::CONNECTION_FAILURE,
        SqlState::CONNECTION_DOES_NOT_EXIST,
        SqlState::SQLCLIENT_UNABLE_TO_ESTABLISH_SQLCONNECTION,
        SqlState::SQLSERVER_REJECTED_ESTABLISHMENT_OF_SQLCONNECTION,
    ]
    .contains(code)
}

/// The driver also wraps SCRAM/protocol parse errors (InvalidInput/InvalidData) and
/// session-attribute refusals (PermissionDenied) in I/O errors; those are not transient.
fn transport(kind: ErrorKind) -> bool {
    !matches!(
        kind,
        ErrorKind::InvalidInput
            | ErrorKind::InvalidData
            | ErrorKind::PermissionDenied
            | ErrorKind::Unsupported
    )
}
