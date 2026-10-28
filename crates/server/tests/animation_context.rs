mod support;
use serde_json::{json, Value};
use support::Server;

const ENTER: &str = r#"{"name":"enter","duration":{"tokenId":"motion-fast"},"easing":"ease-out","keyframes":[{"offset":0,"opacity":0,"translateY":12}]}"#;

/// A frame with two animated nodes (three animations), a frame without any, and a token
/// (`motion-fast`) that only animations reference.
async fn fixture(s: &Server) -> String {
    let p = s.project().await;
    let enter: Value = serde_json::from_str(ENTER).unwrap();
    let ops = json!([
        {"op":"upsert_token","token":{"id":"color-brand","name":"Brand","type":"color","value":{"default":"#1a7aff","themes":{"dark":"#000000"}}}},
        {"op":"upsert_token","token":{"id":"motion-fast","name":"Fast","type":"number","value":200}},
        {"op":"upsert_token","token":{"id":"unused","name":"Unused","type":"number","value":1}},
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"name":"Home"}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":320,"height":200,"fill":"#ffffff","layout":{"mode":"column","padding":16,"gap":8}}},
        {"op":"create_node","id":"card","kind":"rectangle","parentId":"frame","props":{"width":120,"height":40,"fill":"#eeeeee","animations":[enter,
            {"name":"lift","on":"hover","duration":200,"keyframes":[{"scale":1.02}]}]}},
        {"op":"create_node","id":"label","kind":"text","parentId":"frame","props":{"text":"Hello","width":200,"height":"hug","fontId":"font_bundled","fontSize":16,"lineHeight":24,
            "animations":[{"name":"pulse","on":"click","duration":800,"iterations":"infinite","direction":"alternate","keyframes":[{"fill":{"tokenId":"color-brand"}}]}]}},
        {"op":"create_node","id":"still","kind":"frame","parentId":"page","props":{"width":100,"height":100}}
    ]);
    let r = s.batch(&p, 0, "fixture", ops).await;
    assert_eq!(r["committedRevision"], 1, "{r}");
    p
}

async fn pages(s: &Server, mut args: Value) -> Vec<Value> {
    let mut pages = Vec::new();
    loop {
        let page = s.call("get_design_context", args.clone()).await;
        assert!(page.get("error").is_none(), "{page}");
        let next = page["nextCursor"].clone();
        pages.push(page);
        if next.is_null() {
            return pages;
        }
        args["cursor"] = next;
    }
}

#[tokio::test]
async fn animated_lists_every_animation_on_every_page_and_only_when_present() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let expected = json!([
        {"ref":"card:enter","on":"load"},
        {"ref":"card:lift","on":"hover"},
        {"ref":"label:pulse","on":"click"}
    ]);
    let paged = pages(&s, json!({"projectId":p,"frameId":"frame","limit":1})).await;
    assert!(paged.len() > 1, "fixture must page");
    for page in &paged {
        assert_eq!(page["animated"], expected);
    }
    let still = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"still"}),
        )
        .await;
    assert!(still.get("animated").is_none(), "{still}");
    assert!(still.get("error").is_none(), "{still}");
    let subtree = s
        .call("get_subtree", json!({"projectId":p,"nodeId":"card"}))
        .await;
    let enter: Value = serde_json::from_str(ENTER).unwrap();
    assert_eq!(subtree["nodes"][0]["props"]["animations"][0], enter);
}

#[tokio::test]
async fn resolved_animations_follow_the_theme_and_compact_keeps_their_tokens() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let label = |pages: &[Value]| {
        pages
            .iter()
            .flat_map(|p| p["nodes"].as_array().unwrap().clone())
            .find(|n| n["id"] == "label")
            .unwrap()
    };
    for (theme, color) in [("default", "#1a7aff"), ("dark", "#000000")] {
        let read = pages(&s, json!({"projectId":p,"frameId":"frame","theme":theme})).await;
        let node = label(&read);
        assert_eq!(
            node["resolvedProps"]["animations"][0]["keyframes"][0]["fill"],
            color
        );
        assert_eq!(
            node["props"]["animations"][0]["keyframes"][0]["fill"],
            json!({"tokenId":"color-brand"})
        );
    }
    let compact = pages(&s, json!({"projectId":p,"frameId":"frame","compact":true})).await;
    let tokens: Vec<Value> = compact
        .iter()
        .flat_map(|p| p["tokens"].as_array().unwrap().clone())
        .map(|t| t["id"].clone())
        .collect();
    assert!(tokens.contains(&json!("motion-fast")), "{tokens:?}");
    assert!(tokens.contains(&json!("color-brand")), "{tokens:?}");
    assert!(!tokens.contains(&json!("unused")), "{tokens:?}");
}

#[tokio::test]
async fn wrong_keyframe_key_suggests_the_animation_key() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let op = json!({"op":"update_node","id":"card","props":{"animations":[
        {"name":"enter","duration":400,"keyframes":[{"y":12}]}]}});
    let r = s.batch(&p, 1, "bad-y", json!([op])).await;
    let e = &r["error"];
    assert_eq!(e["code"], "VALIDATION_FAILED", "{r}");
    assert_eq!(e["field"], "props.animations[0].keyframes[0].y");
    assert_eq!(e["suggestion"], "translateY");
    assert_eq!(e["nodeId"], "card");
    assert_eq!(e["nodeKind"], "rectangle");
    assert!(e["allowedKeys"]
        .as_array()
        .unwrap()
        .contains(&json!("translateX")));
}

#[tokio::test]
async fn props_reference_instructions_and_skills_declare_animations() {
    let s = Server::start().await;
    let read = |uri: &'static str| {
        let s = &s;
        async move {
            let r = s.rpc("resources/read", json!({ "uri": uri })).await;
            r["contents"][0]["text"].as_str().unwrap().to_string()
        }
    };
    let props = read("skill://opencanvas/props.md").await;
    for line in props.lines().filter(|l| l.starts_with("- `")) {
        let page = line.starts_with("- `page`");
        assert_eq!(line.contains("`animations`"), !page, "{line}");
    }
    assert!(props.contains("## Animations"));
    assert!(props.contains("| `translateY` |"));
    let author = read("skill://opencanvas/author-frame.md").await;
    assert!(author.contains("## Animation"));
    let init = s
        .rpc(
            "initialize",
            json!({"protocolVersion":"2025-11-25","capabilities":{},
        "clientInfo":{"name":"t","version":"1"}}),
        )
        .await;
    let text = init["instructions"].as_str().unwrap();
    assert!(text.contains("props.animations"));
    assert!(!text.contains("says nothing about interaction, animation"));
}

/// Static renders show the resting (authored) props: adding animations changes no pixel.
#[tokio::test]
async fn static_render_ignores_animations() {
    let s = Server::start().await;
    let p = s.project().await;
    s.batch(&p, 0, "design", support::design()).await;
    let png = |revision: u64| {
        let s = &s;
        let p = p.clone();
        async move {
            let job = s
                .call(
                    "render_frame",
                    json!({"projectId":p,"revision":revision,
                "frameId":"frame","viewport":{"width":320,"height":200}}),
                )
                .await;
            let job = s.wait_job(job["jobId"].as_str().unwrap()).await;
            assert_eq!(job["status"], "ready", "{job}");
            let url = job["imageUrl"].as_str().unwrap().to_string();
            let bytes = s
                .client
                .get(url)
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap();
            (bytes.to_vec(), job["sceneKey"].clone())
        }
    };
    let (before, _) = png(1).await;
    let ops = json!([
        {"op":"update_node","id":"frame","props":{"animations":[{"name":"enter","duration":400,"keyframes":[{"offset":0,"opacity":0}]}]}},
        {"op":"update_node","id":"text","props":{"animations":[{"name":"spin","duration":400,"keyframes":[{"rotate":90,"fill":"#ff0000"}]}]}}
    ]);
    assert_eq!(s.batch(&p, 1, "animate", ops).await["committedRevision"], 2);
    let (after, _) = png(2).await;
    assert!(before.len() > 8);
    assert_eq!(before, after, "static PNG changed after adding animations");
}
