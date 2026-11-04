use opencanvas_server::{Config, RenderBackend};

#[test]
fn backend_names_are_exact_and_default_is_chromium() {
    assert_eq!(RenderBackend::default(), RenderBackend::Chromium);
    assert_eq!("chromium".parse(), Ok(RenderBackend::Chromium));
    assert_eq!("rust".parse(), Ok(RenderBackend::Rust));
    for value in ["", "Rust", "RUST", "rust ", " chromium", "native"] {
        assert!(value.parse::<RenderBackend>().is_err());
    }
}

// Isolate PATH changes in a child test process, never in this test runner.
#[cfg(unix)]
#[test]
fn factories_select_backend_before_runtime_probes() {
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};

    const CHILD: &str = "OPENCANVAS_TEST_BACKEND_CONFIG_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let bin = tempfile::tempdir().unwrap();
        let node = bin.path().join("node");
        fs::write(&node, "#!/bin/sh\necho invoked > \"$NODE_PROBE_MARKER\"\n").unwrap();
        fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "factories_select_backend_before_runtime_probes"])
            .env(CHILD, "1")
            .env("PATH", bin.path())
            .env("NODE_PROBE_MARKER", bin.path().join("invoked"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let workspace = tempfile::tempdir().unwrap();
    let marker = std::env::var_os("NODE_PROBE_MARKER").unwrap();
    let marker = std::path::PathBuf::from(marker);
    let mut native = Config::for_backend(workspace.path().into(), 42, RenderBackend::Rust);
    assert_eq!(native.render_backend, RenderBackend::Rust);
    assert_eq!(native.base_url, "http://127.0.0.1:42");
    assert_eq!(native.node, "node");
    assert!(!marker.exists(), "native factory must not invoke Node");

    // Preserve the public worker executable setting without involving it in native identity.
    native.node = marker.with_file_name("node").to_string_lossy().into_owned();
    fs::write(
        workspace.path().join("Cargo.lock"),
        "runtime file is not compiled input",
    )
    .unwrap();
    let same = Config::for_backend(workspace.path().into(), 99, RenderBackend::Rust);
    let other = Config::for_backend(workspace.path().join("not-present"), 0, RenderBackend::Rust);
    assert_eq!(native.pipeline_identity, same.pipeline_identity);
    assert_eq!(native.pipeline_identity, other.pipeline_identity);
    assert_eq!(native.pipeline_identity.len(), 64);
    assert!(native
        .pipeline_identity
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit()));
    assert!(
        !marker.exists(),
        "native fingerprint must not probe runtime binaries"
    );

    let default = Config::new(workspace.path().into(), 42);
    let chromium = Config::for_backend(workspace.path().into(), 42, RenderBackend::Chromium);
    assert!(
        marker.exists(),
        "marker must catch the existing Chromium probe"
    );
    assert_eq!(default.render_backend, RenderBackend::Chromium);
    assert_eq!(default.pipeline_identity, chromium.pipeline_identity);
    assert_ne!(native.pipeline_identity, default.pipeline_identity);
}
