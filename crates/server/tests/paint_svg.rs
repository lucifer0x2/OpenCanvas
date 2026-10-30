//! Production SVG of paint properties: gradients with alpha stops and oversized per-corner radii
//! are exported and compared in Chromium with the CSS reference and the canvas
//! (`spikes/pipeline/paint-svg-offline.mjs`); shadow and stroke are named per file, never
//! dropped silently.
#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

fn rect(id: &str, x: u32, y: u32, props: Value) -> Value {
    let mut p = json!({"x":x,"y":y,"width":200,"height":60});
    p.as_object_mut()
        .unwrap()
        .extend(props.as_object().unwrap().clone());
    let parent = if y < 300 { "art" } else { "frame" };
    json!({"op":"create_node","id":id,"kind":"rectangle","parentId":parent,"props":p})
}

async fn svg(s: &Server, p: &str, node: &str) -> (Value, String) {
    let args = json!({"projectId":p,"nodeIds":[node],"settings":[{"format":"svg"}]});
    let item = s.call("export_nodes", args).await["exports"][0].clone();
    assert_eq!(item["status"], "ready", "{item}");
    let file = item["files"][0].clone();
    let url = file["url"].as_str().unwrap();
    let markup = s
        .client
        .get(url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    (file, markup)
}

#[tokio::test]
async fn gradients_and_radii_export_like_css_and_unexported_effects_are_named() {
    let s = Server::start().await;
    let p = s.project().await;
    let linear = |stops: Value| json!({"type":"linear","angle":90,"stops":stops});
    let ops = json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":480,"height":400,"fill":"#fff","layout":{"mode":"free"}}},
        {"op":"create_node","id":"art","kind":"frame","parentId":"frame","props":{"x":0,"y":0,"width":480,"height":300,"fill":"#fff","layout":{"mode":"free"}}},
        // Review point A: straight-alpha SVG interpolation would gray these out.
        rect("clear", 20, 20, json!({"fill":linear(json!([[0,"#f00"],[1,"#0000"]])),"radius":[24,4,12,0]})),
        rect("blue0", 260, 20, json!({"fill":linear(json!([[0,"#f00"],[1,"#00f0"]]))})),
        rect("mixed", 20, 100, json!({"fill":linear(json!([[0,"#ff000080"],[1,"#0000ffcc"]]))})),
        rect("radial", 260, 100, json!({"fill":{"type":"radial","center":[0.3,0.4],"radius":60,"stops":[[0,"#f00"],[0.5,"#0f08"],[1,"#00f0"]]}})),
        rect("ellipse", 20, 180, json!({"fill":{"type":"radial","stops":[[0,"#f005"],[1,"#00f0"]]},"radius":8})),
        // Review point B for SVG geometry: f = 80/105 on every corner.
        rect("corners", 260, 180, json!({"height":80,"fill":"#334155","radius":[100,20,80,5]})),
        rect("shadowed", 20, 320, json!({"fill":"#fff","shadow":[{"y":4,"blur":12,"color":"#0004"}]})),
        rect("stroked", 260, 320, json!({"fill":"#fff","stroke":{"color":"#000","width":[1,2,3,4]}}))
    ]);
    assert_eq!(s.batch(&p, 0, "paint", ops).await["committedRevision"], 1);

    let (file, markup) = svg(&s, &p, "art").await;
    assert!(file.get("diagnostics").is_none(), "{file}");
    assert!(
        markup.contains("<linearGradient") && markup.contains("<radialGradient"),
        "{markup}"
    );
    assert!(
        markup.contains("<path d=\"M"),
        "per-corner radii are a path"
    );
    // A stop fading to `#0000` keeps its neighbor's hue at zero opacity (premultiplied CSS).
    assert!(markup.contains("<stop offset=\"1\" stop-color=\"#ff0000\" stop-opacity=\"0\"/>"));

    let (file, _) = svg(&s, &p, "frame").await;
    assert_eq!(
        file["diagnostics"],
        json!(["SVG_STROKE_NOT_EXPORTED", "SVG_SHADOW_NOT_EXPORTED"]),
        "{file}"
    );
    let (file, _) = svg(&s, &p, "shadowed").await;
    assert_eq!(
        file["diagnostics"],
        json!(["SVG_SHADOW_NOT_EXPORTED"]),
        "{file}"
    );
    let (file, _) = svg(&s, &p, "stroked").await;
    assert_eq!(
        file["diagnostics"],
        json!(["SVG_STROKE_NOT_EXPORTED"]),
        "{file}"
    );

    let rendered = s.call("render_frame", json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":480,"height":400},"includeScene":true})).await;
    let job = if rendered["status"] == "pending" {
        s.wait_job(rendered["jobId"].as_str().unwrap()).await
    } else {
        rendered
    };
    assert_eq!(job["status"], "ready", "{job}");
    let (_, art) = svg(&s, &p, "art").await;
    let svg_path = s.root.path().join("paint.svg");
    let scene_path = s.root.path().join("scene.json");
    std::fs::write(&svg_path, art).unwrap();
    std::fs::write(&scene_path, serde_json::to_vec(&job["scene"]).unwrap()).unwrap();
    let checked = std::process::Command::new("node")
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .arg("spikes/pipeline/paint-svg-offline.mjs")
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
