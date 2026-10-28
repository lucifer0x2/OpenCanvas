#[allow(dead_code)] // `get` and `wait_job` are unused here.
mod support;
use serde_json::{json, Value};
use support::{design, Server};

const PROPS: &str = "skill://opencanvas/props.md";

fn kind_line(kind: &str, keys: &Value) -> String {
    let keys: Vec<String> = keys
        .as_array()
        .unwrap()
        .iter()
        .map(|key| format!("`{}`", key.as_str().unwrap()))
        .collect();
    format!("- `{kind}`: {}", keys.join(", "))
}

/// A wrong key names itself, its node and kind, the kind's keys and a suggestion, and
/// the published props.md lists exactly those keys for that kind.
#[tokio::test]
async fn unsupported_prop_error_matches_published_reference() {
    let s = Server::start().await;
    let listed = s.rpc("resources/list", json!({})).await;
    assert!(listed["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["uri"] == PROPS && r["mimeType"] == "text/markdown"));
    let read = s.rpc("resources/read", json!({"uri": PROPS})).await;
    let doc = read["contents"][0]["text"].as_str().unwrap().to_string();

    let p = s.project().await;
    s.batch(&p, 0, "design", design()).await;
    let cases = [
        (
            "rectangle",
            json!({"width":10,"height":10,"cornerRadius":6}),
            "cornerRadius",
            Some("radius"),
        ),
        (
            "text",
            json!({"text":"a","fontId":"font_bundled","fontSize":12,"lineHeight":16,"color":"#000"}),
            "color",
            Some("fill"),
        ),
        (
            "rectangle",
            json!({"width":10,"height":10,"assetId":"x"}),
            "assetId",
            None,
        ),
    ];
    for (index, (kind, props, key, suggestion)) in cases.into_iter().enumerate() {
        let op =
            json!({"op":"create_node","id":"probe","kind":kind,"parentId":"frame","props":props});
        let result = s.batch(&p, 1, &format!("probe-{index}"), json!([op])).await;
        let error = &result["error"];
        assert_eq!(error["code"], "VALIDATION_FAILED", "{result}");
        assert_eq!(error["field"], format!("props.{key}"));
        assert_eq!(error["opIndex"], 0);
        assert_eq!(error["nodeId"], "probe");
        assert_eq!(error["nodeKind"], kind);
        assert_eq!(error.get("suggestion").and_then(Value::as_str), suggestion);
        let line = kind_line(kind, &error["allowedKeys"]);
        assert!(doc.contains(&line), "{line} missing from props.md");
    }
}
