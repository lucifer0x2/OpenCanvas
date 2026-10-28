mod support;
use serde_json::json;
use support::{design, Server};

#[tokio::test]
async fn pagination_binds_revision_scope_filters_and_preserves_theme_values() {
    let s = Server::start().await;
    let p = s.project().await;
    let mut ops = design();
    ops.as_array_mut().unwrap().extend([
        json!({"op":"upsert_token","token":{"id":"base","name":"Base","type":"color","value":{"default":"#ffffff","themes":{"night":"#111111"}}}}),
        json!({"op":"upsert_token","token":{"id":"alias","name":"Alias","type":"color","value":{"aliasId":"base"}}}),
        json!({"op":"update_node","id":"frame","props":{"fill":{"tokenId":"alias"}}})
    ]);
    let result = s.batch(&p, 0, "initial", ops).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    let mut req = json!({"projectId":p,"limit":1,"depth":0});
    let mut ids = Vec::new();
    let first = s.call("get_outline", req.clone()).await;
    assert!(first["truncated"].as_bool().unwrap());
    assert!(first["nextCursor"].is_string());
    let cursor = first["nextCursor"].clone();
    for _ in 0..10 {
        let page = s.call("get_outline", req.clone()).await;
        for n in page["nodes"].as_array().unwrap() {
            ids.push(n["id"].as_str().unwrap().to_owned());
        }
        if page["truncated"] == false {
            break;
        }
        req["cursor"] = page["nextCursor"].clone();
    }
    assert_eq!(ids, vec!["page", "frame", "text"]);
    let changed = s
        .call(
            "get_outline",
            json!({"projectId":p,"limit":2,"depth":0,"cursor":cursor}),
        )
        .await;
    assert_eq!(changed["error"]["code"], "INVALID_CURSOR");
    let context=s.call("get_design_context",json!({"projectId":p,"frameId":"frame","theme":"night","viewport":{"width":320,"height":200}})).await;
    assert_eq!(context["nodes"][0]["props"]["fill"]["tokenId"], "alias");
    assert_eq!(context["nodes"][0]["resolvedProps"]["fill"], "#111111");
    assert!(context["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["resolvedValue"] == "#111111"));
    let unsupported = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"frame","theme":"missing"}),
        )
        .await;
    assert_eq!(unsupported["error"]["code"], "UNSUPPORTED_THEME");
    let mut req = json!({"projectId":p,"frameId":"frame","theme":"night","limit":1});
    let mut resources = 0;
    let mut nodes = 0;
    for _ in 0..20 {
        let page = s.call("get_design_context", req.clone()).await;
        nodes += page["nodes"].as_array().unwrap().len();
        for field in ["tokens", "fonts", "styles", "assets"] {
            resources += page[field].as_array().unwrap().len();
        }
        assert!(serde_json::to_vec(&page).unwrap().len() <= 65536);
        if page["truncated"] == false {
            break;
        }
        req["cursor"] = page["nextCursor"].clone();
    }
    assert_eq!(nodes, 2);
    assert_eq!(resources, 3);
    s.batch(
        &p,
        1,
        "update",
        json!([{"op":"update_node","id":"text","props":{"text":"changed"}}]),
    )
    .await;
    assert_eq!(
        s.call(
            "get_outline",
            json!({"projectId":p,"limit":1,"depth":0,"cursor":cursor})
        )
        .await["error"]["code"],
        "INVALID_CURSOR"
    );
}
