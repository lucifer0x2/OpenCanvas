#[allow(dead_code)]
mod support;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use support::Server;

#[tokio::test]
async fn typography_reaches_scene_context_and_production_portable_svg_without_mutating_source() {
    let s = Server::start().await;
    let p = s.project().await;
    let font = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");
    let imported = s.call("import_asset", json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"font","mediaType":"font/ttf","base64":STANDARD.encode(font),"font":{"id":"lato","family":"Portable Text","weight":400,"style":"normal","license":"OFL-1.1"}})).await;
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    let source = "Straße sao chép\nhello-world o'neil";
    let created = s.batch(&p, 1, "text", json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":400,"height":240,"fill":"#fff"}},
        {"op":"upsert_style","style":{"id":"type","name":"Type","kind":"text","value":{"fontId":"lato","fontSize":24,"lineHeight":32,"letterSpacing":-0.5,"textAlign":"center","textTransform":"capitalize","textDecoration":"underline"}}},
        {"op":"create_node","id":"center","kind":"text","parentId":"frame","props":{"text":source,"styleId":"type","x":10,"y":10,"width":380,"fill":"#182230"}},
        {"op":"create_node","id":"right","kind":"text","parentId":"frame","props":{"text":"Sao chép Ấp\nsecond line ỹ","fontId":"font_bundled","fontSize":14,"lineHeight":20,"letterSpacing":1.25,"textAlign":"right","textDecoration":"line-through","x":10,"y":90,"width":300,"fill":"#182230"}},
        {"op":"create_node","id":"nowrap","kind":"text","parentId":"frame","props":{"text":"customers\ncustomers","fontId":"lato","fontSize":14,"lineHeight":20,"textWrap":"nowrap","textAlign":"center","width":20,"x":10,"y":160,"fill":"#182230"}}
    ])).await;
    assert_eq!(created["committedRevision"], 2, "{created}");
    let before = s.state.snapshot(p.clone()).await.unwrap();
    let first = s.call("render_frame", json!({"projectId":p,"revision":2,"frameId":"frame","viewport":{"width":400,"height":240}})).await;
    let job = if first["status"] == "pending" {
        s.wait_job(first["jobId"].as_str().unwrap()).await
    } else {
        first
    };
    assert_eq!(job["status"], "ready", "{job}");
    let scene = &job["scene"];
    assert_eq!(scene["nodes"][1]["lines"][0]["text"], "Straße Sao Chép");
    assert_eq!(scene["nodes"][1]["lines"][1]["text"], "Hello-World O'neil");
    assert_eq!(scene["nodes"][3]["lines"].as_array().unwrap().len(), 2);
    assert_eq!(scene["nodes"][3]["lines"][0]["x"], 0);
    let context = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"frame","compact":false}),
        )
        .await;
    let center = context["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "center")
        .unwrap();
    assert_eq!(center["props"]["text"], source);
    let exported = s
        .call(
            "export_nodes",
            json!({"projectId":p,"nodeIds":["frame"],"settings":[{"format":"svg"}]}),
        )
        .await;
    assert_eq!(exported["exports"][0]["status"], "ready", "{exported}");
    let url = exported["exports"][0]["files"][0]["url"].as_str().unwrap();
    let svg = s
        .client
        .get(url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(svg.contains("letter-spacing=\"-0.5\""));
    assert!(svg.contains("Hello-World O&apos;neil"));
    assert!(svg.contains("data:font/ttf;base64,"));
    let svg_path = s.root.path().join("text.svg");
    let scene_path = s.root.path().join("scene.json");
    std::fs::write(&svg_path, svg).unwrap();
    std::fs::write(&scene_path, serde_json::to_vec(scene).unwrap()).unwrap();
    let checked = std::process::Command::new("node")
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .arg("spikes/pipeline/text-svg-offline.mjs")
        .arg(svg_path)
        .arg(scene_path)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&checked.stdout),
        String::from_utf8_lossy(&checked.stderr)
    );
    assert_eq!(s.state.snapshot(p).await.unwrap(), before);
}
