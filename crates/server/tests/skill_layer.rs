mod support;
mod tool_support;
use serde_json::{json, Value};
use support::Server;

fn decode(text: &str) -> Value {
    let raw = text
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .find(|line| !line.is_empty())
        .unwrap_or(text);
    serde_json::from_str(raw).unwrap_or_else(|_| panic!("Bad MCP response: {text}"))
}

#[tokio::test]
async fn resources_are_advertised_embedded_and_read_only_with_tool_compatibility() {
    let s = Server::start().await;
    let initialize = s
        .client
        .post(format!("{}/mcp", s.url))
        .header("accept", "application/json, text/event-stream")
        .json(
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "protocolVersion":"2025-11-25","capabilities":{},
                "clientInfo":{"name":"capability-check","version":"1"}
            }}),
        )
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let info = decode(&initialize);
    assert!(info["result"]["capabilities"]["resources"].is_object());
    assert!(info["result"]["capabilities"]["tools"].is_object());
    assert!(info["result"]["instructions"]
        .as_str()
        .unwrap()
        .contains("NO_REFLOW_INFORMATION"));
    let resources = s.rpc("resources/list", json!({})).await;
    let listed = resources["resources"].as_array().unwrap();
    assert_eq!(listed.len(), 3);
    assert!(resources["nextCursor"].is_null());
    for (index, (uri, expected)) in [
        (
            "skill://opencanvas/implement-frame.md",
            include_str!("../skills/implement-frame.md"),
        ),
        (
            "skill://opencanvas/author-frame.md",
            include_str!("../skills/author-frame.md"),
        ),
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(listed[index]["uri"], *uri);
        assert_eq!(listed[index]["mimeType"], "text/markdown");
        assert_eq!(listed[index]["size"], expected.len());
        let read = s.rpc("resources/read", json!({"uri":uri})).await;
        assert_eq!(read["contents"].as_array().unwrap().len(), 1);
        assert_eq!(read["contents"][0]["uri"], *uri);
        assert_eq!(read["contents"][0]["mimeType"], "text/markdown");
        assert_eq!(read["contents"][0]["text"], *expected);
    }
    let response = s
        .request(json!({"jsonrpc":"2.0","id":99,"method":"resources/read",
        "params":{"uri":"skill://opencanvas/missing.md"}}))
        .await;
    let missing = decode(&response.text().await.unwrap());
    assert_eq!(missing["error"]["code"], -32002);
    assert!(missing["result"].is_null());
    let tools = s.rpc("tools/list", json!({})).await;
    tool_support::assert_catalog(&tools);
    let p = s.project().await;
    assert!(s.call("list_projects", json!({})).await["projects"]
        .as_array()
        .unwrap()
        .iter()
        .any(|project| project["projectId"] == p));
    assert!(s.root.path().join("test-project/db.sql").is_file());
}

#[tokio::test]
async fn context_diagnostic_uses_complete_authored_frame_scope_on_every_page() {
    let s = Server::start().await;
    let p = s.project().await;
    let result = s.batch(&p, 0, "frames", json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"free","kind":"frame","parentId":"page","props":{"width":960,"height":720}},
        {"op":"create_node","id":"leaf","kind":"rectangle","parentId":"free","props":{"width":100,"height":50,"x":24,"y":32}},
        {"op":"create_node","id":"stacks","kind":"frame","parentId":"page","props":{"width":960,"height":720,"layout":{"mode":"row"}}},
        {"op":"create_node","id":"nested","kind":"frame","parentId":"page","props":{"width":960,"height":720,"layout":{"mode":"free"}}},
        {"op":"create_node","id":"group","kind":"group","parentId":"nested","props":{"width":200,"height":100}},
        {"op":"create_node","id":"deep-stack","kind":"group","parentId":"group","props":{"width":100,"height":50,"layout":{"mode":"column"}}}
    ])).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    let request = json!({"projectId":p,"frameId":"free","viewport":{"width":320,"height":480},
        "properties":["name"],"limit":1});
    let first = s.call("get_design_context", request.clone()).await;
    let diagnostic = &first["diagnostics"][0];
    assert_eq!(first["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(diagnostic["code"], "NO_REFLOW_INFORMATION");
    assert_eq!(diagnostic["frameId"], "free");
    assert_eq!(
        diagnostic["authoredSize"],
        json!({"width":960,"height":720})
    );
    assert!(diagnostic["message"]
        .as_str()
        .unwrap()
        .contains("no reflow information"));
    assert_eq!(first["viewport"], request["viewport"]);
    assert_eq!(first["nodes"][0]["props"], json!({}));
    let mut next = request;
    next["cursor"] = first["nextCursor"].clone();
    let continuation = s.call("get_design_context", next).await;
    assert_eq!(continuation["diagnostics"], first["diagnostics"]);
    assert_eq!(continuation["nodes"][0]["id"], "leaf");
    for frame in ["stacks", "nested"] {
        let context = s
            .call(
                "get_design_context",
                json!({"projectId":p,"frameId":frame,
            "properties":["name"],"limit":1,"depth":0}),
            )
            .await;
        assert_eq!(context["nodes"].as_array().unwrap().len(), 1);
        assert_eq!(context["diagnostics"], json!([]), "{context}");
    }
    assert!(s
        .call("get_subtree", json!({"projectId":p,"nodeId":"free"}))
        .await
        .get("diagnostics")
        .is_none());
}
