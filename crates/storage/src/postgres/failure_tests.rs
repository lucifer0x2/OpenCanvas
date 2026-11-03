use super::failure::{classify, Failure};
use ::postgres::error::SqlState;
use std::io::ErrorKind;

fn state(code: SqlState) -> Failure {
    classify(Some(&code), false, None)
}

#[test]
fn contention_states_stay_busy() {
    for code in [
        SqlState::LOCK_NOT_AVAILABLE,
        SqlState::QUERY_CANCELED,
        SqlState::T_R_DEADLOCK_DETECTED,
    ] {
        assert_eq!(state(code), Failure::Busy);
    }
}

#[test]
fn shutdown_overload_and_connection_states_are_unavailable() {
    for code in [
        SqlState::ADMIN_SHUTDOWN,
        SqlState::CRASH_SHUTDOWN,
        SqlState::CANNOT_CONNECT_NOW,
        SqlState::TOO_MANY_CONNECTIONS,
        SqlState::CONNECTION_EXCEPTION,
        SqlState::CONNECTION_FAILURE,
        SqlState::CONNECTION_DOES_NOT_EXIST,
        SqlState::SQLCLIENT_UNABLE_TO_ESTABLISH_SQLCONNECTION,
        SqlState::SQLSERVER_REJECTED_ESTABLISHMENT_OF_SQLCONNECTION,
    ] {
        assert_eq!(state(code), Failure::Unavailable);
    }
}

#[test]
fn credential_catalog_and_other_states_fail_permanently() {
    for code in [
        SqlState::INVALID_AUTHORIZATION_SPECIFICATION,
        SqlState::INVALID_PASSWORD,
        SqlState::INSUFFICIENT_PRIVILEGE,
        SqlState::INVALID_CATALOG_NAME,
        SqlState::UNIQUE_VIOLATION,
        SqlState::SYNTAX_ERROR,
        SqlState::DISK_FULL,
        SqlState::from_code("XX999"),
    ] {
        assert_eq!(state(code), Failure::Failed);
    }
    // A SQLSTATE wins over transport hints carried by the same error.
    let auth = SqlState::INVALID_PASSWORD;
    assert_eq!(
        classify(Some(&auth), true, Some(ErrorKind::ConnectionReset)),
        Failure::Failed
    );
}

#[test]
fn connection_level_failures_without_state_are_unavailable() {
    assert_eq!(classify(None, true, None), Failure::Unavailable);
    for kind in [
        ErrorKind::ConnectionRefused,
        ErrorKind::ConnectionReset,
        ErrorKind::ConnectionAborted,
        ErrorKind::NotConnected,
        ErrorKind::BrokenPipe,
        ErrorKind::TimedOut,
        ErrorKind::UnexpectedEof,
        ErrorKind::HostUnreachable,
        ErrorKind::NetworkUnreachable,
        ErrorKind::Other,
    ] {
        assert_eq!(classify(None, false, Some(kind)), Failure::Unavailable);
    }
}

#[test]
fn driver_parse_and_permission_errors_without_state_fail() {
    assert_eq!(classify(None, false, None), Failure::Failed);
    for kind in [
        ErrorKind::InvalidInput,
        ErrorKind::InvalidData,
        ErrorKind::PermissionDenied,
        ErrorKind::Unsupported,
    ] {
        assert_eq!(classify(None, false, Some(kind)), Failure::Failed);
    }
}
