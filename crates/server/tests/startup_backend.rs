use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Output, Stdio},
    sync::mpsc,
    time::Duration,
};

const SECRET: &str = "backend-secret-must-not-appear";

fn server() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_opencanvas-server"));
    for name in [
        "RENDER_BACKEND",
        "DATABASE_PROVIDER",
        "DATABASE_URL",
        "PORT",
        "OPENCANVAS_DATA_ROOT",
        "HOME",
    ] {
        command.env_remove(name);
    }
    command
}

fn failure(output: Output, message: &str) {
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    for stream in [&output.stdout, &output.stderr] {
        assert!(!String::from_utf8_lossy(stream).contains(SECRET));
    }
}

#[test]
fn render_backend_env_requires_an_exact_supported_value() {
    for value in ["", "Rust", "RUST", "rust ", " chromium", SECRET] {
        failure(
            server().env("RENDER_BACKEND", value).output().unwrap(),
            "RENDER_BACKEND must be chromium or rust",
        );
    }
    for value in ["chromium", "rust"] {
        failure(
            server().env("RENDER_BACKEND", value).output().unwrap(),
            "Data root is required",
        );
    }
}

#[cfg(unix)]
#[test]
fn non_unicode_backend_env_is_rejected_without_echoing_it() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    failure(
        server()
            .env("RENDER_BACKEND", OsString::from_vec(vec![0xff]))
            .output()
            .unwrap(),
        "RENDER_BACKEND must be chromium or rust",
    );
}

#[test]
fn cli_backend_is_validated_and_overrides_env() {
    for value in ["rust", "chromium"] {
        failure(
            server()
                .env("RENDER_BACKEND", SECRET)
                .args(["--render-backend", value])
                .output()
                .unwrap(),
            "Data root is required",
        );
    }
    failure(
        server().arg("--render-backend").output().unwrap(),
        "--render-backend needs chromium or rust",
    );
    failure(
        server()
            .args(["--render-backend", SECRET])
            .output()
            .unwrap(),
        "--render-backend must be chromium or rust",
    );
}

#[test]
fn help_ignores_invalid_backend_env_even_after_other_options() {
    let output = server()
        .env("RENDER_BACKEND", SECRET)
        .env("DATABASE_PROVIDER", SECRET)
        .env("PORT", SECRET)
        .args(["--render-backend", SECRET, "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("RENDER_BACKEND") && help.contains("--render-backend"));
    assert!(!help.contains(SECRET));
}

struct RunningServer(Child);

impl Drop for RunningServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn native_startup_with_empty_path_uses_only_temporary_data_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    let mut process = RunningServer(
        server()
            .env("PATH", "")
            .env("RENDER_BACKEND", "rust")
            .arg("--data-root")
            .arg(&root)
            .args(["--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = process.0.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = sender.send(result);
    });
    let ready = receiver.recv_timeout(Duration::from_secs(20));
    // Stop even on timeout; startup success is not evidence of native render dispatch.
    drop(process);
    reader.join().unwrap();
    let ready = ready.expect("server startup timed out").unwrap();
    let address = ready
        .strip_prefix("OpenCanvas http://127.0.0.1:")
        .expect("server did not announce readiness");
    let port: u16 = address.split(' ').next().unwrap().parse().unwrap();
    assert_ne!(port, 0);
    assert!(root.is_dir());
    assert!(!root.join("db.sql").exists());
}
