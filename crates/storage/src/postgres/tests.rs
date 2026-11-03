use super::*;
use std::{net::TcpListener, time::Instant};

#[test]
fn invalid_database_configuration_is_client_safe() {
    let config = PostgresConfig::new("host=secret.invalid password=DO_NOT_ECHO unknown=1");
    let error = match PostgresBackend::with_config(config) {
        Ok(_) => panic!("invalid driver option accepted"),
        Err(error) => error,
    };
    assert_eq!(error.code, "VALIDATION_FAILED");
    assert!(!error.to_string().contains("DO_NOT_ECHO"));
    let mut config = PostgresConfig::new("host=127.0.0.1");
    config.pool_size = 0;
    assert!(config.driver().is_err());
}

#[test]
fn unresponsive_handshake_has_a_finite_operation_wait() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (connection, _) = listener.accept().unwrap();
        std::thread::sleep(Duration::from_millis(350));
        drop(connection);
    });
    let mut config = PostgresConfig::new(format!(
        "host=127.0.0.1 port={} user=test dbname=test sslmode=disable",
        address.port()
    ));
    config.pool_size = 1;
    config.operation_timeout = Duration::from_millis(100);
    config.connect_timeout = Duration::from_millis(100);
    let start = Instant::now();
    let error = match PostgresBackend::with_config(config) {
        Ok(_) => panic!("unresponsive server connected"),
        Err(error) => error,
    };
    assert_eq!(error.code, "STORAGE_UNAVAILABLE");
    assert!(start.elapsed() < Duration::from_secs(1));
    server.join().unwrap();
}

#[test]
fn refused_connection_is_retryable_and_client_safe() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut config = PostgresConfig::new(format!(
        "host=127.0.0.1 port={port} user=SECRET_USER password=DO_NOT_ECHO dbname=SECRET_DB sslmode=disable"
    ));
    config.pool_size = 1;
    config.connect_timeout = Duration::from_millis(500);
    config.operation_timeout = Duration::from_secs(5);
    let start = Instant::now();
    let error = match PostgresBackend::with_config(config) {
        Ok(_) => panic!("closed port connected"),
        Err(error) => error,
    };
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(error.code, "STORAGE_UNAVAILABLE");
    assert!(error.retryable);
    let text = format!(
        "{error} {error:?} {}",
        serde_json::to_string(&error).unwrap()
    );
    for secret in [
        "SECRET_USER",
        "DO_NOT_ECHO",
        "SECRET_DB",
        "127.0.0.1",
        &port.to_string(),
    ] {
        assert!(!text.contains(secret), "error echoes {secret}");
    }
}
