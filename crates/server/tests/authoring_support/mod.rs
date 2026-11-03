//! Parse the published examples rather than a second copy of their JSON.
use crate::support::Server;
use serde_json::{json, Value};

pub async fn guidance(s: &Server) -> String {
    let resource = s
        .rpc(
            "resources/read",
            json!({"uri":"skill://opencanvas/author-frame.md"}),
        )
        .await;
    let text = resource["contents"][0]["text"].as_str().unwrap();
    assert_eq!(text, include_str!("../../skills/author-frame.md"));
    text.to_owned()
}

pub fn example(text: &str, marker: &str) -> Value {
    let (_, after) = text.split_once(&format!("<!-- {marker} -->")).unwrap();
    let (_, after) = after.split_once("```json\n").unwrap();
    let (body, _) = after.split_once("\n```").unwrap();
    serde_json::from_str(body).unwrap()
}

pub fn arguments(call: &Value, project: &str) -> Value {
    let mut args = call["arguments"].clone();
    assert_eq!(args["projectId"], "PROJECT_ID");
    args["projectId"] = json!(project);
    args
}

pub async fn invoke(s: &Server, project: &str, call: &Value) -> Value {
    let result = s
        .call(call["name"].as_str().unwrap(), arguments(call, project))
        .await;
    assert!(result.get("error").is_none(), "{result}");
    result
}

pub async fn discover(s: &Server) {
    let listed = s.rpc("tools/list", json!({})).await;
    for tool in ["apply_batch", "apply_batch_file"] {
        let found = listed["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == tool)
            .unwrap();
        assert_eq!(found["inputSchema"]["type"], "object");
        assert_eq!(
            found["inputSchema"]["properties"]["dryRun"]["type"],
            "boolean"
        );
    }
    let capabilities = s
        .rpc("resources/read", json!({"uri":"opencanvas://capabilities"}))
        .await;
    assert!(capabilities["contents"][0]["text"].is_string());
    let props = s
        .rpc(
            "resources/read",
            json!({"uri":"skill://opencanvas/props.md"}),
        )
        .await;
    assert!(props["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("theme"));
}

pub async fn subtree(s: &Server, project: &str, node: &str) -> Value {
    let result = s
        .call("get_subtree", json!({"projectId":project,"nodeId":node}))
        .await;
    assert!(result.get("error").is_none(), "{result}");
    result
}
