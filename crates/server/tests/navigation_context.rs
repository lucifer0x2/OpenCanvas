#[allow(dead_code)] // `get`, `wait_job` and `design` are unused here.
mod support;
use serde_json::{json, Value};
use support::Server;

fn frame(id: &str, name: &str, extra: Value) -> Value {
    let mut props = json!({"name":name,"width":390,"height":844});
    props
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    json!({"op":"create_node","id":id,"kind":"frame","parentId":"p1","props":props})
}

fn button(id: &str, parent: &str, links: Value) -> Value {
    json!({"op":"create_node","id":id,"kind":"rectangle","parentId":parent,"props":{"width":80,"height":40,"links":links}})
}

/// home: flowStart Main and three links (navigate, overlay, back); done links back to home.
async fn fixture(s: &Server) -> String {
    let p = s.project().await;
    let ops = json!([
        {"op":"create_node","id":"p1","kind":"page","parentId":null,"props":{"name":"App"}},
        frame("home", "Home", json!({"flowStart":"Main","layout":{"mode":"column"}})),
        frame("search", "Search", json!({})),
        frame("db-menu", "Open databases", json!({})),
        frame("done", "Done", json!({})),
        frame("plain", "Plain", json!({})),
        button("home-search", "home", json!([{"to":"search"}])),
        button("home-menu", "home", json!([{"to":"db-menu","action":"overlay"}])),
        button("home-back", "home", json!([{"action":"back"}])),
        button("home-deco", "home", json!([])),
        button("done-btn", "done", json!([{"to":"home"}])),
    ]);
    let result = s.batch(&p, 0, "fixture", ops).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    p
}

async fn context(s: &Server, p: &str, frame: &str, extra: Value) -> Value {
    let mut args = json!({"projectId":p,"frameId":frame});
    args.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    s.call("get_design_context", args).await
}

#[tokio::test]
async fn design_context_lists_outgoing_and_incoming_links() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let home = context(&s, &p, "home", json!({})).await;
    assert_eq!(
        home["navigation"],
        json!({
            "flowStart":"Main",
            "out":[
                {"from":"home-search","on":"click","action":"navigate","to":"search","toName":"Search"},
                {"from":"home-menu","on":"click","action":"overlay","to":"db-menu","toName":"Open databases"},
                {"from":"home-back","on":"click","action":"back"}
            ],
            "in":[{"from":"done-btn","fromFrame":"done","fromFrameName":"Done","on":"click","action":"navigate"}]
        })
    );
    let search = context(&s, &p, "search", json!({})).await;
    assert_eq!(
        search["navigation"],
        json!({"in":[{"from":"home-search","fromFrame":"home","fromFrameName":"Home","on":"click","action":"navigate"}]})
    );
    let plain = context(&s, &p, "plain", json!({})).await;
    assert!(plain.get("navigation").is_none(), "{plain}");
    let compact = context(&s, &p, "home", json!({"compact":true})).await;
    assert_eq!(compact["navigation"], home["navigation"]);
}

#[tokio::test]
async fn navigation_is_first_page_metadata_and_capped() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let first = context(&s, &p, "home", json!({"limit":2})).await;
    assert!(first["navigation"].is_object());
    let cursor = first["nextCursor"].clone();
    assert!(cursor.is_string(), "{first}");
    let next = context(&s, &p, "home", json!({"limit":2,"cursor":cursor})).await;
    assert!(next["nodes"].as_array().is_some_and(|n| !n.is_empty()));
    assert!(next.get("navigation").is_none(), "{next}");

    let mut ops = vec![frame("many", "Many", json!({}))];
    ops.extend((0..60).map(|i| button(&format!("m{i}"), "many", json!([{"to":"home"}]))));
    let result = s.batch(&p, 1, "many", Value::Array(ops)).await;
    assert_eq!(result["committedRevision"], 2, "{result}");
    let many = context(&s, &p, "many", json!({"limit":1})).await;
    assert_eq!(many["navigation"]["out"].as_array().unwrap().len(), 50);
    assert_eq!(many["navigation"]["truncated"], true);
    let home = context(&s, &p, "home", json!({"limit":1})).await;
    assert_eq!(home["navigation"]["in"].as_array().unwrap().len(), 50);
    assert_eq!(home["navigation"]["truncated"], true);
}

#[tokio::test]
async fn deleting_a_target_prunes_links_and_reports_sources() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let ops = json!([{"op":"delete_subtree","id":"search"}]);
    let result = s.batch(&p, 1, "drop", ops).await;
    assert_eq!(result["committedRevision"], 2, "{result}");
    assert!(result["changedIds"]
        .as_array()
        .unwrap()
        .contains(&json!("home-search")));
    let changes = s
        .call("get_changes", json!({"projectId":p,"sinceRevision":1}))
        .await;
    assert!(changes.to_string().contains("home-search"), "{changes}");
    let home = context(&s, &p, "home", json!({})).await;
    assert_eq!(home["navigation"]["out"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn props_reference_and_instructions_publish_links() {
    let s = Server::start().await;
    let read = s
        .rpc(
            "resources/read",
            json!({"uri":"skill://opencanvas/props.md"}),
        )
        .await;
    let doc = read["contents"][0]["text"].as_str().unwrap();
    assert!(
        doc.contains("| `links` | frame, group, rectangle, text, image |"),
        "{doc}"
    );
    assert!(doc.contains("| `flowStart` | frame |"));
    let p = fixture(&s).await;
    for (i, key) in ["reactions", "onClick"].into_iter().enumerate() {
        let ops = json!([{"op":"update_node","id":"home-deco","props":{key:[]}}]);
        let e = s.batch(&p, 1, &format!("alias-{i}"), ops).await;
        assert_eq!(e["error"]["suggestion"], "links", "{e}");
    }
    let init = s.rpc("initialize", json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"1"}})).await;
    let text = init["instructions"].as_str().unwrap();
    assert!(text.contains("props.links") && text.contains("flowStart"));
    assert!(!text.contains("says nothing about interaction, animation, routing"));
}
