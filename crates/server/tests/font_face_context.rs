#[allow(dead_code)]
mod support;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use support::Server;

const REGULAR: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Bold.ttf");
const ITALIC: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Italic.ttf");

async fn fixture(s: &Server) -> String {
    let p = s.project().await;
    for (revision, id, weight, style, bytes) in [
        (0, "regular", 400, "normal", REGULAR),
        (1, "bold", 700, "normal", BOLD),
        (2, "italic", 400, "italic", ITALIC),
    ] {
        let result = s.call("import_asset", json!({"projectId":p,"expectedRevision":revision,"idempotencyKey":id,"mediaType":"font/ttf","base64":STANDARD.encode(bytes),"font":{"id":id,"family":"Static Lato","weight":weight,"style":style,"license":"OFL-1.1"}})).await;
        assert_eq!(result["committedRevision"], revision + 1, "{result}");
    }
    let mut operations = support::design();
    operations[2]["props"]
        .as_object_mut()
        .unwrap()
        .remove("fontId");
    operations[2]["props"]["styleId"] = json!("body");
    operations.as_array_mut().unwrap().insert(2,json!({"op":"upsert_style","style":{"id":"body","kind":"text","name":"Body","value":{"fontFamily":"Static Lato","fontWeight":400,"fontStyle":"normal"}}}));
    let created = s.batch(&p, 3, "design", operations).await;
    assert_eq!(created["committedRevision"], 4, "{created}");
    p
}
async fn render(s: &Server, p: &str, revision: u64) -> Value {
    let initial = s.call("render_frame",json!({"projectId":p,"revision":revision,"frameId":"frame","viewport":{"width":320,"height":200}})).await;
    let job = if initial["status"] == "pending" {
        s.wait_job(initial["jobId"].as_str().unwrap()).await
    } else {
        initial
    };
    assert_eq!(job["status"], "ready", "{job}");
    job
}
async fn png(s: &Server, job: &Value) -> Vec<u8> {
    s.client
        .get(job["imageUrl"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap()
        .to_vec()
}

#[tokio::test]
async fn real_static_faces_reach_context_scene_png_and_portable_svg() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let mut pngs = Vec::new();
    let mut keys = Vec::new();
    let mut svg_paths = Vec::new();
    for (revision, id, weight, style, bytes) in [
        (4, "regular", 400, "normal", REGULAR),
        (5, "bold", 700, "normal", BOLD),
        (6, "italic", 400, "italic", ITALIC),
    ] {
        if revision > 4 {
            let updated = s.batch(&p,revision-1,&format!("style-{id}"),json!([{"op":"upsert_style","style":{"id":"body","kind":"text","name":"Body","value":{"fontFamily":"Static Lato","fontWeight":weight,"fontStyle":style}}}])).await;
            assert_eq!(updated["committedRevision"], revision, "{updated}");
        }
        let context = s
            .call(
                "get_design_context",
                json!({"projectId":p,"frameId":"frame","compact":true}),
            )
            .await;
        let node = context["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "text")
            .unwrap();
        assert!(node["props"].get("fontId").is_none());
        assert_eq!(node["resolvedProps"]["fontId"], id);
        assert_eq!(context["fonts"].as_array().unwrap().len(), 1, "{context}");
        assert_eq!(context["fonts"][0]["id"], id);
        let downloaded = s
            .client
            .get(format!("{}/api/v1/projects/{p}/fonts/{id}/download", s.url))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(downloaded.as_ref(), bytes);
        let job = render(&s, &p, revision).await;
        assert_eq!(job["scene"]["diagnostics"], json!([]));
        let scene_node = job["scene"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "text")
            .unwrap();
        assert_eq!(scene_node["fontId"], id, "{scene_node}");
        assert_eq!(job["scene"]["fonts"][0]["weight"], weight);
        assert_eq!(job["scene"]["fonts"][0]["style"], style);
        assert_eq!(
            job["scene"]["fonts"][0]["hash"],
            context["fonts"][0]["contentHash"]
        );
        pngs.push(png(&s, &job).await);
        keys.push(job["sceneKey"].clone());
        let exports = s
            .call(
                "export_nodes",
                json!({"projectId":p,"nodeIds":["frame"],"settings":[{"format":"svg"}]}),
            )
            .await;
        assert_eq!(exports["exports"][0]["status"], "ready", "{exports}");
        let svg = s
            .client
            .get(exports["exports"][0]["files"][0]["url"].as_str().unwrap())
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(svg.contains(&STANDARD.encode(bytes)));
        assert!(!svg.contains("Static Lato") && !svg.contains("sans-serif"));
        let path = s.root.path().join(format!("{id}.svg"));
        std::fs::write(&path, svg).unwrap();
        svg_paths.push(path);
    }
    assert_ne!(pngs[0], pngs[1]);
    assert_ne!(pngs[0], pngs[2]);
    assert_ne!(pngs[1], pngs[2]);
    assert_ne!(keys[0], keys[1]);
    assert_ne!(keys[1], keys[2]);
    let offline = std::process::Command::new("node")
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .arg("spikes/pipeline/font-face-svg-offline.mjs")
        .args(&svg_paths)
        .output()
        .unwrap();
    assert!(
        offline.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&offline.stdout),
        String::from_utf8_lossy(&offline.stderr)
    );
    // Missing imported face is atomic, including a preceding valid change.
    let bad = s.batch(&p,6,"missing",json!([{"op":"update_node","id":"frame","props":{"name":"rollback"}},{"op":"update_node","id":"text","props":{"fontWeight":700}}])).await;
    assert_eq!(bad["error"]["field"], "fontStyle", "{bad}");
    assert_eq!(s.state.snapshot(p).await.unwrap().revision, 6);
}

#[tokio::test]
async fn compact_closes_over_theme_faces_and_instance_style_definitions() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let changed = s.batch(&p,4,"themes",json!([
        {"op":"upsert_token","token":{"id":"weight","name":"Weight","type":"number","value":{"default":400,"themes":{"bold":700}}}},
        {"op":"upsert_style","style":{"id":"body","kind":"text","name":"Body","value":{"fontFamily":"Static Lato","fontWeight":{"tokenId":"weight"}}}},
        {"op":"upsert_style","style":{"id":"italic","kind":"text","name":"Italic","value":{"fontFamily":"Static Lato","fontStyle":"italic"}}},
        {"op":"update_node","id":"frame","props":{"component":{"name":"Card"}}},
        {"op":"create_node","id":"host","kind":"frame","parentId":"page","props":{"width":400,"height":300}},
        {"op":"create_node","id":"instance","kind":"instance","parentId":"host","props":{"componentId":"frame","overrides":{"text":{"styleId":"italic"}}}}
    ])).await;
    assert_eq!(changed["committedRevision"], 5, "{changed}");
    for theme in ["default", "bold"] {
        let context = s.call("get_design_context",json!({"projectId":p,"frameId":"host","compact":true,"theme":theme,"instances":"expand"})).await;
        let node = context["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == "instance/text")
            .unwrap();
        assert_eq!(node["resolvedProps"]["fontId"], "italic", "{context}");
        assert_eq!(context["fonts"].as_array().unwrap().len(), 1);
        let reference = s
            .call(
                "get_design_context",
                json!({"projectId":p,"frameId":"host","compact":true,"theme":theme}),
            )
            .await;
        let ids: std::collections::BTreeSet<_> = reference["fonts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            std::collections::BTreeSet::from(["regular", "bold", "italic"]),
            "{reference}"
        );
        assert!(context["fonts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["contentHash"]
                .as_str()
                .is_some_and(|hash| hash.len() == 64)));
    }
}
