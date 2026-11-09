use std::process::{Command, Output};

const SECRET: &str = "test-secret-must-not-appear";

fn server() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_opencanvas-server"));
    for variable in [
        "DATABASE_PROVIDER",
        "DATABASE_URL",
        "PORT",
        "OPENCANVAS_DATA_ROOT",
        "HOME",
        "RENDER_BACKEND",
    ] {
        command.env_remove(variable);
    }
    for variable in LIMITS {
        command.env_remove(variable);
    }
    command
}

fn assert_failure(output: Output, message: &str) {
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    assert!([&output.stdout, &output.stderr]
        .into_iter()
        .all(|stream| !String::from_utf8_lossy(stream).contains(SECRET)));
}

#[test]
fn database_provider_requires_an_exact_supported_value() {
    for provider in ["", "PostgreSQL", "postgresql ", SECRET] {
        assert_failure(
            server()
                .env("DATABASE_PROVIDER", provider)
                .output()
                .unwrap(),
            "DATABASE_PROVIDER must be sqlite or postgresql",
        );
    }
}

#[test]
fn postgres_requires_a_nonblank_database_url() {
    let mut command = server();
    command.env("DATABASE_PROVIDER", "postgresql");
    assert_failure(command.output().unwrap(), "DATABASE_URL must be set");
    for url in ["", " \t "] {
        assert_failure(
            command.env("DATABASE_URL", url).output().unwrap(),
            "DATABASE_URL must be set",
        );
    }
}

#[test]
fn invalid_postgres_url_fails_without_echoing_credentials_or_opening_sqlite() {
    let temp = tempfile::tempdir().unwrap();
    let blocked = temp.path().join("file");
    std::fs::File::create(&blocked).unwrap();
    let root = blocked.join("sqlite");
    assert_failure(
        server()
            .env("DATABASE_PROVIDER", "postgresql")
            .env(
                "DATABASE_URL",
                format!("postgresql://user:{SECRET}@[broken"),
            )
            .env("PORT", "0")
            .env("OPENCANVAS_DATA_ROOT", &root)
            .output()
            .unwrap(),
        "PostgreSQL startup failed",
    );
    assert!(!root.exists());
}

#[test]
fn port_env_is_validated_and_cli_wins() {
    for port in ["", "-1", "65536", SECRET] {
        assert_failure(
            server().env("PORT", port).output().unwrap(),
            "PORT must be a number from 0 to 65535",
        );
    }
    assert_failure(
        server()
            .env("PORT", SECRET)
            .args(["--port", "0"])
            .output()
            .unwrap(),
        "Data root is required",
    );
}

#[test]
fn public_port_must_differ_from_effective_local_port() {
    assert_failure(
        server()
            .env("PORT", "4321")
            .args([
                "--public-viewer-port",
                "4321",
                "--public-viewer-origin",
                "https://example.test",
            ])
            .output()
            .unwrap(),
        "Local and public viewer ports must differ",
    );
}

#[test]
fn help_works_with_invalid_env_and_unknown_options_still_fail() {
    let output = server()
        .env("PORT", SECRET)
        .env("DATABASE_PROVIDER", SECRET)
        .env("DATABASE_URL", SECRET)
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("DATABASE_PROVIDER") && help.contains("PORT"));
    assert!(!help.contains(SECRET) && output.stderr.is_empty());
    assert_failure(
        server().arg("--unknown").output().unwrap(),
        "Unknown server option",
    );
}

const LIMITS: [&str; 5] = [
    "DATABASE_POOL_SIZE",
    "DATABASE_CONNECT_TIMEOUT_MS",
    "DATABASE_STATEMENT_TIMEOUT_MS",
    "DATABASE_LOCK_TIMEOUT_MS",
    "DATABASE_OPERATION_TIMEOUT_MS",
];

/// A PostgreSQL URL whose port was just released, so connecting is refused at once.
fn refused_postgres() -> Command {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut command = server();
    command
        .env("DATABASE_PROVIDER", "postgresql")
        .env(
            "DATABASE_URL",
            format!("postgresql://user:{SECRET}@127.0.0.1:{port}/db?sslmode=disable"),
        )
        .env("PORT", "0");
    command
}

#[test]
fn invalid_postgres_limits_name_the_variable_without_echoing_the_url() {
    for name in LIMITS {
        for value in ["", "0", "-1", "1.5", "33ms", "2147483648", SECRET] {
            let expected = format!("{name} must be an integer from 1 to");
            assert_failure(
                refused_postgres().env(name, value).output().unwrap(),
                &expected,
            );
        }
    }
    assert_failure(
        refused_postgres()
            .env("DATABASE_POOL_SIZE", "33")
            .output()
            .unwrap(),
        "DATABASE_POOL_SIZE must be an integer from 1 to 32",
    );
}

#[test]
fn valid_postgres_limits_and_defaults_reach_the_connection_attempt() {
    assert_failure(
        refused_postgres().output().unwrap(),
        "PostgreSQL startup failed",
    );
    let mut command = refused_postgres();
    for (name, value) in LIMITS
        .into_iter()
        .zip(["32", "200", "60000", "1", "2147483647"])
    {
        command.env(name, value);
    }
    let output = command.output().unwrap();
    assert!(!String::from_utf8_lossy(&output.stderr).contains("must be an integer"));
    assert_failure(output, "PostgreSQL startup failed");
}

#[test]
fn sqlite_ignores_postgres_limits() {
    assert_failure(
        server()
            .env("DATABASE_PROVIDER", "sqlite")
            .env("DATABASE_POOL_SIZE", "0")
            .env("DATABASE_STATEMENT_TIMEOUT_MS", SECRET)
            .output()
            .unwrap(),
        "Data root is required",
    );
}
