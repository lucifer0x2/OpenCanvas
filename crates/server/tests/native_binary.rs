//! Execute the actual Rust binary; PATH and workspace have no Node/layout worker.
#[allow(dead_code)]
mod export_support;
#[allow(dead_code)]
mod native_support;
use base64::{engine::general_purpose::STANDARD, Engine};
use native_support::Mcp;
use serde_json::json;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn built_binary_empty_path_creates_design_renders_and_exports_without_node() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("empty-workspace");
    std::fs::create_dir(&workspace).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_opencanvas-server"));
    for name in [
        "DATABASE_PROVIDER",
        "DATABASE_URL",
        "PORT",
        "OPENCANVAS_DATA_ROOT",
        "HOME",
    ] {
        command.env_remove(name);
    }
    let mut running = Running(
        command
            .env("PATH", "")
            .env("RENDER_BACKEND", "rust")
            .env("OPENCANVAS_LAYOUT_BIN", workspace.join("no-layout-step"))
            .current_dir(&workspace)
            .arg("--data-root")
            .arg(root.path().join("data"))
            .arg("--workspace")
            .arg(&workspace)
            .args(["--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = running.0.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = sender.send(result);
    });
    let ready = receiver
        .recv_timeout(Duration::from_secs(20))
        .expect("native binary startup timed out")
        .unwrap();
    reader.join().unwrap();
    let url = ready
        .strip_prefix("OpenCanvas ")
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned();
    let s = Mcp::connect(url).await;
    let p = s.project().await;
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[11, 22, 33, 255])
            .unwrap();
    }
    let imported = s
        .call(
            "import_asset",
            json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"image",
        "mediaType":"image/png","base64":STANDARD.encode(png)}),
        )
        .await;
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    let snapshot = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    let mut design = export_support::design();
    design.as_array_mut().unwrap().push(
        json!({"op":"create_node","id":"image","kind":"image","parentId":"frame",
        "props":{"assetId":snapshot["assets"][0]["id"],"x":80,"y":80,"width":20,"height":20}}),
    );
    assert_eq!(s.batch(&p, 1, design).await["committedRevision"], 2);
    let frame = s.render(&p, 2, "default").await;
    assert_eq!(frame["status"], "ready", "{frame}");
    assert!(frame.get("error").is_none(), "{frame}");
    assert_eq!(frame["provenance"]["engine"], "native");
    let pixels = s
        .client
        .get(frame["imageUrl"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let pixels = export_support::decode(&pixels);
    assert_eq!((pixels.0, pixels.1), (320, 200));
    assert_eq!(export_support::pixel(&pixels, 30, 30), [239, 68, 68, 255]);
    assert_eq!(export_support::pixel(&pixels, 90, 90), [11, 22, 33, 255]);
    let exported = s
        .call(
            "export_nodes",
            json!({"projectId":p,"nodeIds":["box","label"],
        "settings":[{"width":96},{"format":"svg"}]}),
        )
        .await;
    for item in exported["exports"].as_array().unwrap() {
        assert_eq!(item["status"], "ready", "{exported}");
        for file in item["files"].as_array().unwrap() {
            let response = s
                .client
                .get(file["url"].as_str().unwrap())
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 200);
            let bytes = response.bytes().await.unwrap();
            if file["format"] == "png" {
                assert_eq!(export_support::decode(&bytes).0, 96);
            } else {
                assert!(std::str::from_utf8(&bytes).unwrap().contains("<svg"));
            }
        }
    }
    assert_eq!(std::fs::read_dir(&workspace).unwrap().count(), 0);
}
