#[allow(dead_code)]
mod support;
use serde_json::json;
use support::Server;

async fn fixture(s: &Server) -> String {
    let p = s.project().await;
    let ops = json!([
        {"op":"upsert_token","token":{"id":"brand","name":"Brand","type":"color","value":{"default":"#123456","themes":{"night":"#abcdef"}}}},
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":320,"height":200}},
        {"op":"create_node","id":"button","kind":"rectangle","parentId":"frame","props":{"width":120,"height":40,"fill":"#123456"}}
    ]);
    assert_eq!(s.batch(&p, 0, "fixture", ops).await["committedRevision"], 1);
    p
}

#[tokio::test]
async fn states_context_authored_themed_differing_compact_resources_and_filters() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let states =
        json!({"hover":{"fill":{"tokenId":"brand"},"opacity":1},"pressed":{"opacity":0.5}});
    let applied = s
        .batch(
            &p,
            1,
            "states",
            json!([{"op":"update_node","id":"button","props":{"states":states}}]),
        )
        .await;
    assert_eq!(applied["committedRevision"], 2, "{applied}");
    let tree = s
        .call("get_subtree", json!({"projectId":p,"nodeId":"button"}))
        .await;
    assert_eq!(tree["nodes"][0]["props"]["states"], states);
    for compact in [false, true] {
        let context=s.call("get_design_context",json!({"projectId":p,"frameId":"frame","theme":"night","statesMode":"resolved","compact":compact})).await;
        assert!(context.get("error").is_none(), "{context}");
        let nodes = context["nodes"].as_array().unwrap();
        assert!(nodes[0].get("resolvedStates").is_none());
        assert_eq!(nodes[1]["props"]["states"], states);
        assert_eq!(
            nodes[1]["resolvedStates"],
            json!({"hover":{"fill":"#abcdef"},"pressed":{"opacity":0.5}})
        );
        assert_eq!(context["tokens"][0]["id"], "brand");
    }
    let default = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"frame","statesMode":"resolved"}),
        )
        .await;
    assert_eq!(default["nodes"][1]["resolvedStates"]["hover"], json!({}));
    let filtered = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"frame","statesMode":"resolved","properties":["fill"]}),
        )
        .await;
    assert!(filtered["nodes"][1].get("resolvedStates").is_none());
    for bad in [json!({"hover":{"width":99}}), json!({"foo":{}})] {
        let e = s
            .batch(
                &p,
                2,
                "bad",
                json!([{"op":"update_node","id":"button","props":{"states":bad}}]),
            )
            .await;
        assert_eq!(e["error"]["code"], "VALIDATION_FAILED");
        assert!(e["error"]["allowedKeys"].is_array());
        assert_eq!(
            s.get(&format!("/api/v1/projects/{p}/snapshot")).await["revision"],
            2
        );
    }
}

#[tokio::test]
async fn states_none_preserves_exact_bytes_and_cursor_scope_mode_is_bound() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let args = json!({"projectId":p,"frameId":"frame","limit":1});
    let omitted = s.call("get_design_context", args.clone()).await;
    assert_eq!(omitted["truncated"], true);
    let mut explicit = args.clone();
    explicit["statesMode"] = json!("none");
    let none = s.call("get_design_context", explicit.clone()).await;
    assert_eq!(
        serde_json::to_vec(&omitted).unwrap(),
        serde_json::to_vec(&none).unwrap()
    );
    assert!(omitted["nodes"][0].get("resolvedStates").is_none());
    explicit["cursor"] = omitted["nextCursor"].clone();
    assert!(s
        .call("get_design_context", explicit.clone())
        .await
        .get("error")
        .is_none());
    explicit["statesMode"] = json!("resolved");
    let wrong = s.call("get_design_context", explicit).await;
    assert_eq!(wrong["error"]["code"], "INVALID_CURSOR", "{wrong}");
}

#[tokio::test]
async fn states_schema_and_instructions_growth_stays_bounded() {
    let s = Server::start().await;
    let current = s.rpc("tools/list", json!({})).await;
    let mut before = current.clone();
    for tool in before["tools"].as_array_mut().unwrap() {
        if tool["name"] == "render_frame" {
            tool["inputSchema"]["properties"]["state"]["enum"] = json!(["default"]);
        }
        if tool["name"] == "get_design_context" {
            tool["inputSchema"]["properties"]
                .as_object_mut()
                .unwrap()
                .remove("statesMode");
            let old=tool["description"].as_str().unwrap().replace(" statesMode:resolved adds resolvedStates with only visual overrides differing from default; none (default) omits it.","");
            tool["description"] = json!(old);
        }
    }
    let current_bytes = serde_json::to_vec(&current).unwrap().len();
    let baseline_bytes = serde_json::to_vec(&before).unwrap().len();
    eprintln!(
        "tools/list baseline={baseline_bytes}, current={current_bytes}, delta={}",
        current_bytes - baseline_bytes
    );
    assert!(current_bytes - baseline_bytes <= 400);
    let p = fixture(&s).await;
    let invalid = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"frame","statesMode":"all"}),
        )
        .await;
    assert_eq!(invalid["error"]["code"], "INVALID_REQUEST");
    let init=s.rpc("initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"1"}})).await;
    let instructions = init["instructions"].as_str().unwrap();
    assert!(!instructions.contains("Only interaction state 'default'"));
    assert!(instructions.contains("props.states"));
}
