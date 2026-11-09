use crate::native_layout_support::Server;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn file_hash(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}

pub async fn project(
    server: &Server,
    out: &Path,
    fixture: &str,
    snapshot: Value,
    cases: &mut Vec<Value>,
) {
    let project = snapshot["projectId"].as_str().unwrap();
    let revision = snapshot["revision"].as_u64().unwrap();
    let frames: Vec<_> = snapshot["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["kind"] == "frame")
        .collect();
    assert!(!frames.is_empty(), "fixture has frames");
    for frame in frames {
        let frame_id = frame["id"].as_str().unwrap();
        let authored = (
            frame["props"]["width"].clone(),
            frame["props"]["height"].clone(),
        );
        for (label, width, height) in [
            ("authored", authored.0, authored.1),
            ("desktop", json!(1440), json!(900)),
            ("tablet", json!(768), json!(1024)),
            ("mobile", json!(390), json!(844)),
        ] {
            let request = json!({"projectId":project,"revision":revision,"frameId":frame_id,
                "viewport":{"width":width,"height":height},"state":"default","scale":0.1});
            let pending = server.call("render_frame", request).await;
            let id = pending["jobId"].as_str().expect("real native job ID");
            let job = server.wait(id).await;
            assert_eq!(job["status"], "ready", "{job}");
            // Persist the actual HTTP bytes. serde_json's default decimal parser may
            // shift a binary64 by one ULP; a Rust Value reserialization is not the wire oracle.
            let native_file = format!("{fixture}-{frame_id}-{label}.json");
            let raw = server
                .client
                .get(format!("{}/api/v1/jobs/{id}", server.url))
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .bytes()
                .await
                .unwrap();
            std::fs::write(out.join(&native_file), raw).unwrap();
            let scene = &job["scene"];
            assert!(scene["nodes"].is_array(), "{job}");
            assert_eq!(scene["provenance"]["engine"], "native");
            assert_eq!(scene["provenance"]["frameId"], frame_id);
            assert_eq!(scene["provenance"]["revision"], revision);
            for (axis, expected) in [("width", &width), ("height", &height)] {
                assert_eq!(
                    scene["provenance"]["viewport"][axis].as_f64(),
                    expected.as_f64()
                );
            }
            assert_eq!(scene["provenance"]["sourceSceneKey"], job["sceneKey"]);
            let mut fonts = Vec::new();
            for font in scene["fonts"].as_array().unwrap() {
                let bytes = server
                    .client
                    .get(font["url"].as_str().unwrap())
                    .send()
                    .await
                    .unwrap()
                    .error_for_status()
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap();
                let hash = format!("{:x}", Sha256::digest(&bytes));
                assert_eq!(hash, font["hash"].as_str().unwrap());
                if font["id"] == "font_bundled" {
                    assert_eq!(
                        hash, "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d",
                        "pinned Noto Sans original fixture bytes"
                    );
                }
                let file = format!("fonts/{hash}.bin");
                std::fs::write(out.join(&file), bytes).unwrap();
                fonts.push(json!({"id":font["id"],"hash":hash,"file":file}));
            }
            // No visible-node filter: authored complete subtree must match order and identity.
            let mut expected = vec![frame_id.to_owned()];
            loop {
                let before = expected.len();
                for node in snapshot["nodes"].as_array().unwrap() {
                    if node["parentId"]
                        .as_str()
                        .is_some_and(|p| expected.iter().any(|id| id == p))
                    {
                        let id = node["id"].as_str().unwrap().to_owned();
                        if !expected.contains(&id) {
                            expected.push(id);
                        }
                    }
                }
                if before == expected.len() {
                    break;
                }
            }
            let mut actual: Vec<_> = scene["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n["id"].as_str().unwrap().to_owned())
                .collect();
            expected.sort();
            actual.sort();
            assert_eq!(actual, expected, "complete authored subtree");
            cases.push(json!({"label":format!("{fixture}/{frame_id}/{label}"),
                "frameId":frame_id,"width":width,"height":height,
                "snapshotFile":format!("snapshot-{fixture}.json"),
                "nativeFile":native_file,"fonts":fonts}));
        }
    }
}
