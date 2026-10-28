//! Fixture and helpers for compact_context.rs: one frame that exercises every reference kind,
//! a second frame with resources of its own, and 300 unreferenced filler tokens.
use crate::support::Server;
use serde_json::{json, Value};

pub const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=";
pub const FILLER_TOKENS: usize = 300;

fn token(id: &str, kind: &str, value: Value) -> Value {
    json!({"op":"upsert_token","token":{"id":id,"name":id,"type":kind,"value":value}})
}
fn style(id: &str, kind: &str, value: Value) -> Value {
    json!({"op":"upsert_style","style":{"id":id,"name":id,"kind":kind,"value":value}})
}
fn node(id: &str, kind: &str, parent: Option<&str>, props: Value) -> Value {
    json!({"op":"create_node","id":id,"kind":kind,"parentId":parent,"props":props})
}

/// Returns `(projectId, assetId)`. Frame `card` references, directly or transitively:
/// tokens ink, size-l, brand, brand-night (night branch alias), radius-alias, radius-base
/// (default alias) and space (layout padding); styles heading (text) and brand-fill (fill
/// style used as a fill reference); font_bundled through the text style; the imported asset.
pub async fn fixture(s: &Server) -> (String, String) {
    let p = s.project().await;
    let import = s.call("import_asset", json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"image","mediaType":"image/png","base64":PNG})).await;
    assert!(import.get("error").is_none(), "{import}");
    let snapshot = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    let asset = snapshot["assets"][0]["id"].as_str().unwrap().to_owned();
    let mut ops = vec![
        token("ink", "color", json!("#111111")),
        token("size-l", "dimension", json!(24)),
        token("brand-night", "color", json!("#223344")),
        token(
            "brand",
            "color",
            json!({"default":"#0055ff","themes":{"night":{"aliasId":"brand-night"}}}),
        ),
        token("radius-base", "dimension", json!(12)),
        token(
            "radius-alias",
            "dimension",
            json!({"aliasId":"radius-base"}),
        ),
        token("space", "dimension", json!(16)),
        token("unused-color", "color", json!("#abcdef")),
        style(
            "heading",
            "text",
            json!({"fontId":"font_bundled","fill":{"tokenId":"ink"},"fontSize":{"tokenId":"size-l"},"lineHeight":32}),
        ),
        style("brand-fill", "fill", json!({"tokenId":"brand"})),
        style("unused-style", "fill", json!({"tokenId":"unused-color"})),
        node("page", "page", None, json!({"name":"Home"})),
        node(
            "card",
            "frame",
            Some("page"),
            json!({"name":"Card","width":320,"height":400,"fill":"#ffffff","layout":{"mode":"column","padding":{"tokenId":"space"},"gap":8}}),
        ),
        node(
            "title",
            "text",
            Some("card"),
            json!({"name":"title","styleId":"heading","text":"Tổng quan","width":"fill","height":"hug"}),
        ),
        node(
            "swatch",
            "rectangle",
            Some("card"),
            json!({"name":"swatch","fill":{"styleId":"brand-fill"},"width":40,"height":40}),
        ),
        node(
            "plain",
            "rectangle",
            Some("card"),
            json!({"name":"plain","fill":"#eeeeee","radius":{"tokenId":"radius-alias"},"width":40,"height":40}),
        ),
        node(
            "photo",
            "image",
            Some("card"),
            json!({"name":"photo","assetId":asset,"fit":"fit","width":40,"height":40}),
        ),
        node(
            "other",
            "frame",
            Some("page"),
            json!({"name":"Other","width":100,"height":100,"fill":{"styleId":"unused-style"}}),
        ),
    ];
    for i in 0..FILLER_TOKENS {
        ops.push(token(&format!("filler-{i:03}"), "number", json!(i)));
    }
    let result = s.batch(&p, 1, "fixture", json!(ops)).await;
    assert_eq!(result["committedRevision"], 2, "{result}");
    (p, asset)
}

/// Every page of one read, following nextCursor with identical parameters.
pub async fn pages(s: &Server, mut args: Value) -> Vec<Value> {
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

/// Concatenation of one array field over every page.
pub fn all(pages: &[Value], field: &str) -> Vec<Value> {
    pages
        .iter()
        .flat_map(|p| p[field].as_array().unwrap().iter().cloned())
        .collect()
}

pub fn ids(items: &[Value]) -> Vec<String> {
    let mut ids: Vec<String> = items
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_owned())
        .collect();
    ids.sort();
    ids
}

/// Compact JSON size of every page, the payload the 64 KiB budget governs.
pub fn bytes(pages: &[Value]) -> usize {
    pages
        .iter()
        .map(|p| serde_json::to_vec(p).unwrap().len())
        .sum()
}
