//! `stroke` through MCP: authored refs stay in `props`, `resolvedProps` follow the theme,
//! and compact reads list the tokens a stroke references.
#[allow(dead_code)] // Only `call`, `rpc`, `project` and `batch` are used here.
mod support;
use serde_json::{json, Value};
use support::Server;

fn token(id: &str, kind: &str, value: Value) -> Value {
    json!({"op":"upsert_token","token":{"id":id,"name":id,"type":kind,"value":value}})
}

async fn fixture(s: &Server) -> String {
    let p = s.project().await;
    let ops = json!([
        token("border-night", "color", json!("#344054")),
        token("border", "color", json!({"default":"#D0D5DD","themes":{"night":{"aliasId":"border-night"}}})),
        token("hairline", "dimension", json!(2)),
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"name":"Home"}},
        {"op":"create_node","id":"card","kind":"frame","parentId":"page","props":{"width":200,"height":100,"fill":"#ffffff","layout":{"mode":"column","padding":8}}},
        {"op":"create_node","id":"rule","kind":"rectangle","parentId":"card","props":{"width":100,"height":20,
            "stroke":{"color":{"tokenId":"border"},"width":[0,0,{"tokenId":"hairline"},0]}}},
    ]);
    let result = s.batch(&p, 0, "fixture", ops).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    p
}

fn rule(page: &Value) -> Value {
    page["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "rule")
        .unwrap()
        .clone()
}

#[tokio::test]
async fn design_context_keeps_authored_stroke_and_resolves_it_per_theme() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let authored = json!({"color":{"tokenId":"border"},"width":[0,0,{"tokenId":"hairline"},0]});
    for (theme, color) in [("default", "#D0D5DD"), ("night", "#344054")] {
        let page = s
            .call(
                "get_design_context",
                json!({"projectId":p,"frameId":"card","theme":theme}),
            )
            .await;
        assert!(page.get("error").is_none(), "{page}");
        let node = rule(&page);
        assert_eq!(node["props"]["stroke"], authored);
        assert_eq!(
            node["resolvedProps"]["stroke"],
            json!({"color":color,"width":[0,0,2,0]})
        );
    }
    let compact = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"card","compact":true}),
        )
        .await;
    let tokens: Vec<&str> = compact["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["id"].as_str())
        .collect();
    for id in ["border", "hairline"] {
        assert!(tokens.contains(&id), "{id} missing from {tokens:?}");
    }
}

#[tokio::test]
async fn props_reference_lists_stroke_for_box_kinds_only() {
    let s = Server::start().await;
    let read = s
        .rpc(
            "resources/read",
            json!({"uri":"skill://opencanvas/props.md"}),
        )
        .await;
    let text = read["contents"][0]["text"].as_str().unwrap().to_owned();
    for kind in ["page", "frame", "group", "rectangle", "text", "image"] {
        let line = text
            .lines()
            .find(|l| l.starts_with(&format!("- `{kind}`:")))
            .unwrap_or_else(|| panic!("no line for {kind}"));
        let listed = line.contains("`stroke`");
        assert_eq!(listed, !matches!(kind, "page" | "text"), "{line}");
    }
}
