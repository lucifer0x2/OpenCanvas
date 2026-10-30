//! Production SVG of translucent text whose own paints overlap (decoration across glyphs,
//! negative tracking, tight lines) rasterizes like the canvas: node opacity composites once.
//! `spikes/pipeline/text-opacity-svg-offline.mjs` compares the pixels offline in Chromium.
#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

fn text(id: &str, y: u32, props: Value) -> Value {
    let mut p = json!({"x":10,"y":y,"width":380,"fontId":"font_bundled","fontSize":40,"lineHeight":48,"fill":"#000000","opacity":0.5});
    p.as_object_mut()
        .unwrap()
        .extend(props.as_object().unwrap().clone());
    json!({"op":"create_node","id":id,"kind":"text","parentId":"frame","props":p})
}

#[tokio::test]
async fn translucent_overlapping_text_svg_matches_canvas_offline() {
    let s = Server::start().await;
    let p = s.project().await;
    let ops = json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":400,"height":330,"fill":"#fff","layout":{"mode":"free"}}},
        text("strike", 10, json!({"text":"HHHH","textDecoration":"line-through"})),
        text("under", 80, json!({"text":"gjpqy","textDecoration":"underline"})),
        text("tracking", 150, json!({"text":"HHHH","letterSpacing":-20})),
        text("tight", 220, json!({"text":"gjpq\nHHHH","lineHeight":20}))
    ]);
    assert_eq!(s.batch(&p, 0, "text", ops).await["committedRevision"], 1);
    let first = s.call("render_frame", json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":400,"height":330},"includeScene":true})).await;
    let job = if first["status"] == "pending" {
        s.wait_job(first["jobId"].as_str().unwrap()).await
    } else {
        first
    };
    assert_eq!(job["status"], "ready", "{job}");
    let args = json!({"projectId":p,"nodeIds":["frame"],"settings":[{"format":"svg"}]});
    let exported = s.call("export_nodes", args).await;
    assert_eq!(exported["exports"][0]["status"], "ready", "{exported}");
    let url = exported["exports"][0]["files"][0]["url"].as_str().unwrap();
    let svg = s.client.get(url).send().await.unwrap().text().await;
    let svg = svg.unwrap();
    // Decorated and tight-line text group their elements; one `<text>` keeps its own opacity.
    assert_eq!(svg.matches("<g opacity=\"0.5\">").count(), 3, "{svg}");
    assert_eq!(
        svg.matches(" opacity=\"0.5\" xml:space").count(),
        1,
        "{svg}"
    );
    let svg_path = s.root.path().join("text-opacity.svg");
    let scene_path = s.root.path().join("scene.json");
    std::fs::write(&svg_path, svg).unwrap();
    std::fs::write(&scene_path, serde_json::to_vec(&job["scene"]).unwrap()).unwrap();
    let checked = std::process::Command::new("node")
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .arg("spikes/pipeline/text-opacity-svg-offline.mjs")
        .arg(svg_path)
        .arg(scene_path)
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&checked.stdout);
    println!("{out}");
    assert!(
        checked.status.success(),
        "{out}\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}
