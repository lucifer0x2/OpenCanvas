//! Explicit native-server oracle capture for the generated release WASM browser gate.
#[path = "viewer_wasm_support/capture.rs"]
mod capture;
#[path = "native_support/mcp.rs"]
#[allow(dead_code)]
mod mcp;
#[path = "native_layout_support/mod.rs"]
#[allow(dead_code)]
mod native_layout_support;
use native_layout_support::{fixtures, fonts, workspace, Server};
use opencanvas_server::RenderBackend;
use opencanvas_storage::{Storage, Store};
use serde_json::{json, Value};

fn sample(name: &str, export: &str) -> Vec<Value> {
    let script = format!("import {{{export}}} from './samples/{name}/design.mjs'; console.log(JSON.stringify({export}()));");
    let output = std::process::Command::new("node")
        .current_dir(workspace())
        .args(["--input-type=module", "-e", &script])
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test]
#[ignore = "explicit artifact capture; run with --ignored before the Chromium WASM runner"]
async fn native_server_matrix() {
    let root = tempfile::tempdir().unwrap();
    let store: Storage = Store::open(root.path().join("data")).unwrap().into();
    let server = Server::start(store, RenderBackend::Rust, root.path()).await;
    assert_eq!(server.state.config.render_backend, RenderBackend::Rust);
    let out = workspace().join("artifacts/viewer-wasm-scene");
    std::fs::create_dir_all(out.join("fonts")).unwrap();
    let mut layout = fixtures::repo_wrap();
    layout.extend(fixtures::typography());
    layout.extend(fixtures::missing_control());
    let mut cases = Vec::new();
    for (name, operations) in [
        ("welcome", sample("welcome", "welcomeOperations")),
        (
            "welcome-stacks",
            sample("welcome-stacks", "stacksOperations"),
        ),
        ("native-layout", layout),
    ] {
        let project = server
            .call("create_project", json!({"slug":name,"name":name}))
            .await["projectId"]
            .as_str()
            .unwrap()
            .to_owned();
        let imported = if name == "native-layout" {
            fonts::upload(&server, &project).await
        } else {
            0
        };
        let result = server.batch(&project, imported, json!(operations)).await;
        assert_eq!(result["committedRevision"], imported + 1, "{result}");
        let snapshot = server
            .get(&format!("/api/v1/projects/{project}/snapshot"))
            .await;
        let raw = server
            .client
            .get(format!("{}/api/v1/projects/{project}/snapshot", server.url))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        std::fs::write(out.join(format!("snapshot-{name}.json")), raw).unwrap();
        capture::project(&server, &out, name, snapshot, &mut cases).await;
    }
    assert_eq!(cases.len(), 20, "all five frames at all four sizes");
    std::fs::write(
        out.join("native.json"),
        serde_json::to_vec_pretty(&json!({
            "capture":"MCP render_frame / actual RenderBackend::Rust / temporary SQLite",
            "executableSha256":capture::file_hash(&std::env::current_exe().unwrap()),
            "cases":cases
        }))
        .unwrap(),
    )
    .unwrap();
    eprintln!(
        "NATIVE_CAPTURE comparisons={} output={}",
        cases.len(),
        out.display()
    );
}
