//! CLI validation and dual-listener shutdown with isolated roots and random ports.
use futures_util::StreamExt;
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

const BINARY: &str = env!("CARGO_BIN_EXE_opencanvas-server");
const ORIGIN: &str = "https://canvas.example.ts.net";

#[tokio::test]
async fn public_flags_fail_before_opening_the_data_root() {
    let root = tempfile::tempdir().unwrap();
    for args in [
        vec!["--public-viewer-port", "3848"],
        vec!["--public-viewer-origin", ORIGIN],
        vec![
            "--port",
            "3848",
            "--public-viewer-port",
            "3848",
            "--public-viewer-origin",
            ORIGIN,
        ],
        vec![
            "--public-viewer-port",
            "65536",
            "--public-viewer-origin",
            ORIGIN,
        ],
        vec![
            "--public-viewer-port",
            "0",
            "--public-viewer-origin",
            "http://canvas.example.ts.net",
        ],
    ] {
        let output = Command::new(BINARY)
            .arg("--data-root")
            .arg(root.path())
            .args(&args)
            .output()
            .await
            .unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert!(std::fs::read_dir(root.path()).unwrap().next().is_none());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn both_random_listeners_stop_with_active_sse_on_one_interrupt() {
    let root = tempfile::tempdir().unwrap();
    let mut child = Command::new(BINARY)
        .args([
            "--port",
            "0",
            "--public-viewer-port",
            "0",
            "--public-viewer-origin",
            ORIGIN,
            "--data-root",
        ])
        .arg(root.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let local = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let public = lines.next_line().await.unwrap().unwrap();
    assert!(local.starts_with("OpenCanvas http://127.0.0.1:"), "{local}");
    assert!(
        public.starts_with("Public viewer https://canvas.example.ts.net"),
        "{public}"
    );
    let local_url = local.split_whitespace().nth(1).unwrap();
    let public_addr = public
        .split("bound to ")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();
    let client = reqwest::Client::new();
    let local_stream = client
        .get(format!("{local_url}/api/v1/events"))
        .send()
        .await
        .unwrap();
    let public_stream = client
        .get(format!("http://{public_addr}/api/v1/events"))
        .header("host", "canvas.example.ts.net")
        .send()
        .await
        .unwrap();
    assert_eq!(local_stream.status(), 200);
    assert_eq!(public_stream.status(), 200);
    let mut local_stream = local_stream.bytes_stream();
    let mut public_stream = public_stream.bytes_stream();
    assert!(Command::new("kill")
        .args(["-INT", &child.id().unwrap().to_string()])
        .status()
        .await
        .unwrap()
        .success());
    // Less than the three-second hard-exit fallback: this must be graceful shutdown.
    let status = tokio::time::timeout(Duration::from_millis(2800), child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(status.success());
    assert!(local_stream.next().await.is_none());
    assert!(public_stream.next().await.is_none());
    assert!(client
        .get(format!("{local_url}/api/v1/projects"))
        .send()
        .await
        .is_err());
    assert!(client
        .get(format!("http://{public_addr}/api/v1/projects"))
        .send()
        .await
        .is_err());
}
