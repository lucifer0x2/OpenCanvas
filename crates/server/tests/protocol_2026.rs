//! MCP 2026-07-28 (`server/discover`, no session) requires SEP-2549 cache hints, `ttlMs` and
//! `cacheScope`, on every list and read result; a client such as Claude Code rejects the whole
//! catalog when one is missing. The legacy `initialize` path keeps its result shape and only
//! gains the same two fields, which older revisions accept as extra result properties.
// `root`, `state`, `project`, `batch`, `get`, `wait_job` and `design` are unused here.
#[allow(dead_code)]
mod support;
mod tool_names;
use serde_json::{json, Value};
use support::Server;

const MODERN: &str = "2026-07-28";
const SKILL: &str = "skill://opencanvas/implement-frame.md";

fn decode(text: &str) -> Value {
    let raw = text
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .find(|line| !line.is_empty())
        .unwrap_or(text);
    serde_json::from_str(raw).unwrap_or_else(|_| panic!("Bad MCP response: {text}"))
}

/// One sessionless 2026-07-28 request: the version rides in the header and in `_meta`, which
/// also carries the client capabilities; SEP-2243 headers repeat the method and, for
/// `tools/call` and `resources/read`, the tool name or URI.
async fn modern(s: &Server, id: u64, method: &str, mut params: Value) -> (bool, Value) {
    params["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion": MODERN,
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": {"name": "protocol-2026", "version": "1"}
    });
    let mut request = s
        .client
        .post(format!("{}/mcp", s.url))
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", MODERN)
        .header("mcp-method", method);
    if let Some(name) = params.get("uri").or(params.get("name")) {
        request = request.header("mcp-name", name.as_str().unwrap());
    }
    let response = request
        .json(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
        .send()
        .await
        .unwrap();
    let sessionless = response.headers().get("mcp-session-id").is_none();
    (sessionless, decode(&response.text().await.unwrap()))
}

async fn modern_result(s: &Server, id: u64, method: &str, params: Value) -> Value {
    let (sessionless, value) = modern(s, id, method, params).await;
    assert!(sessionless, "{method} must not open a session");
    assert!(value.get("error").is_none(), "{method}: {value}");
    assert_eq!(value["result"]["resultType"], "complete", "{method}");
    value["result"].clone()
}

/// The server's policy: a fixed catalog for the process lifetime, cached per user.
fn assert_hints(result: &Value, method: &str) {
    assert_eq!(
        result["ttlMs"].as_u64(),
        Some(300_000),
        "{method}: {result}"
    );
    assert_eq!(result["cacheScope"], "private", "{method}: {result}");
}

fn keys(result: &Value) -> Vec<&str> {
    let mut keys: Vec<_> = result
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    keys
}

#[tokio::test]
async fn discover_lifecycle_lists_and_reads_with_cache_hints() {
    let s = Server::start().await;
    let discover = modern_result(&s, 1, "server/discover", json!({})).await;
    assert!(discover["supportedVersions"]
        .as_array()
        .unwrap()
        .contains(&json!(MODERN)));
    let tools = modern_result(&s, 2, "tools/list", json!({})).await;
    assert_hints(&tools, "tools/list");
    let tools = tools["tools"].as_array().unwrap();
    tool_names::assert_listed(tools);
    for tool in tools {
        assert_eq!(tool["inputSchema"]["type"], "object", "{}", tool["name"]);
    }
    let resources = modern_result(&s, 3, "resources/list", json!({})).await;
    assert_hints(&resources, "resources/list");
    // Two skills, props.md and the opencanvas://capabilities JSON resource.
    assert_eq!(resources["resources"].as_array().unwrap().len(), 4);
    let templates = modern_result(&s, 4, "resources/templates/list", json!({})).await;
    assert_hints(&templates, "resources/templates/list");
    assert_eq!(templates["resourceTemplates"], json!([]));
    let read = modern_result(&s, 5, "resources/read", json!({"uri": SKILL})).await;
    assert_hints(&read, "resources/read");
    assert_eq!(
        read["contents"][0]["text"],
        include_str!("../skills/implement-frame.md")
    );
    let call = modern_result(&s, 6, "tools/call", json!({"name": "list_projects"})).await;
    assert_eq!(call["structuredContent"]["projects"], json!([]), "{call}");
    let missing = json!({"uri": "skill://opencanvas/missing.md"});
    let (_, missing) = modern(&s, 7, "resources/read", missing).await;
    assert_eq!(missing["error"]["code"], -32602, "{missing}");
}

#[tokio::test]
async fn legacy_initialize_lifecycle_keeps_shape_and_adds_cache_hints() {
    // `Server::start` runs `initialize` 2025-11-25 and `notifications/initialized`, and `rpc`
    // reuses that session with the 2025-11-25 header. A second `initialize` shows the version
    // the handshake negotiates, which `Server` does not expose.
    let s = Server::start().await;
    let initialize = s
        .client
        .post(format!("{}/mcp", s.url))
        .header("accept", "application/json, text/event-stream")
        .json(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-11-25", "capabilities": {},
            "clientInfo": {"name": "legacy-check", "version": "1"}}}),
        )
        .send()
        .await
        .unwrap();
    let initialize = decode(&initialize.text().await.unwrap());
    assert_eq!(initialize["result"]["protocolVersion"], "2025-11-25");
    let hints = ["cacheScope", "ttlMs"];
    for (method, params, item) in [
        ("tools/list", json!({}), "tools"),
        ("resources/list", json!({}), "resources"),
        ("resources/templates/list", json!({}), "resourceTemplates"),
        ("resources/read", json!({"uri": SKILL}), "contents"),
    ] {
        let legacy = s.rpc(method, params.clone()).await;
        assert_hints(&legacy, method);
        // No `resultType` (stripped for pre-2026 peers) and no `nextCursor`: the item list plus
        // the two hints.
        let mut expected = vec![item, hints[0], hints[1]];
        expected.sort();
        assert_eq!(keys(&legacy), expected, "{method}");
        let current = modern_result(&s, 2, method, params).await;
        assert_eq!(
            legacy[item], current[item],
            "{method} differs between versions"
        );
    }
    let read = s.rpc("resources/read", json!({"uri": SKILL})).await;
    assert_eq!(
        read["contents"][0]["text"],
        include_str!("../skills/implement-frame.md")
    );
}
