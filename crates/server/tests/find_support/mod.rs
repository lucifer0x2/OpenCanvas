//! Fixture and read helpers for find_nodes.rs.
use crate::support::Server;
use serde_json::{json, Value};

fn text(id: &str, parent: &str, copy: &str) -> Value {
    json!({"op":"create_node","id":id,"kind":"text","parentId":parent,"props":{"name":id,"text":copy,"fill":"#111111","width":200,"height":"hug","fontId":"font_bundled","fontSize":14,"lineHeight":20}})
}
fn node(id: &str, kind: &str, parent: Option<&str>, name: &str) -> Value {
    json!({"op":"create_node","id":id,"kind":kind,"parentId":parent,"props":{"name":name,"width":300,"height":200}})
}

/// Two pages, three frames that all have children, a 12-deep chain and one long text.
pub async fn fixture(s: &Server) -> String {
    let p = s.project().await;
    let mut ops = vec![
        node("page-1", "page", None, "Home"),
        node("hero", "frame", Some("page-1"), "Hero Card"),
        text("title", "hero", "Tổng quan doanh thu"),
        node("price-card", "group", Some("hero"), "Price card"),
        text("price", "price-card", "Giá 99.000đ"),
        node("bg", "rectangle", Some("hero"), "Background"),
        node("page-2", "page", None, "Second"),
        node("pricing", "frame", Some("page-2"), "Pricing"),
        text("pricing-text", "pricing", "Bảng giá"),
        text("long", "pricing", &format!("{}needle", "x".repeat(1500))),
        node("footer", "frame", Some("page-2"), "Footer"),
    ];
    let mut parent = "footer".to_owned();
    for i in 1..=10 {
        ops.push(node(&format!("d{i}"), "group", Some(&parent), "level"));
        parent = format!("d{i}");
    }
    ops.push(text("deep-text", &parent, "Deep"));
    let result = s.batch(&p, 0, "fixture", json!(ops)).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    p
}

pub async fn find(s: &Server, args: Value) -> Value {
    let result = s.call("find_nodes", args).await;
    assert!(result.get("error").is_none(), "{result}");
    result
}

/// Every page of one read, following nextCursor with identical parameters.
pub async fn all_pages(s: &Server, tool: &str, mut args: Value) -> Vec<Value> {
    let mut pages = Vec::new();
    loop {
        let page = s.call(tool, args.clone()).await;
        assert!(page.get("error").is_none(), "{page}");
        let next = page["nextCursor"].clone();
        pages.push(page);
        if next.is_null() {
            return pages;
        }
        args["cursor"] = next;
    }
}

pub fn ids(list: &Value) -> Vec<&str> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect()
}
