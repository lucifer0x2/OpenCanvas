#[path = "node_rows_context/measure.rs"]
mod measure;
#[allow(dead_code)]
mod support;
use serde_json::{json, Map, Value};
use support::{design, Server};

const COLUMNS: [&str; 6] = ["id", "parentId", "sortKey", "kind", "props", "extra"];

fn decode_rows(page: &mut Value) {
    let body = page.as_object_mut().unwrap();
    assert_eq!(body.remove("nodeFormat").unwrap(), "rows-v1");
    assert_eq!(body.remove("nodeColumns").unwrap(), json!(COLUMNS));
    let nodes = body["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let row = row.as_array().unwrap();
            assert!((5..=6).contains(&row.len()), "{row:?}");
            let mut node: Map<String, Value> = COLUMNS[..5]
                .iter()
                .zip(row)
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect();
            if let Some(extra) = row.get(5) {
                let extra = extra.as_object().unwrap();
                assert!(!extra.is_empty());
                for (key, value) in extra {
                    assert!(node.insert(key.clone(), value.clone()).is_none());
                }
            }
            Value::Object(node)
        })
        .collect();
    body.insert("nodes".into(), Value::Array(nodes));
}

async fn context(server: &Server, args: Value) -> Value {
    let page = server.call("get_design_context", args).await;
    assert!(page.get("error").is_none(), "{page}");
    page
}

async fn fixture(server: &Server, rectangles: usize) -> String {
    let project = server.project().await;
    let Value::Array(mut ops) = design() else {
        unreachable!()
    };
    ops.extend([
        json!({"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":"#123456"}}),
        json!({"op":"create_node","id":"master","kind":"frame","parentId":"page","props":{"width":40,"height":30,"component":{"name":"Button"},"states":{"hover":{"fill":{"tokenId":"ink"}}}}}),
        json!({"op":"create_node","id":"shape","kind":"rectangle","parentId":"master","props":{"width":10,"height":10,"fill":"#eeeeee"}}),
        json!({"op":"create_node","id":"instance","kind":"instance","parentId":"frame","props":{"componentId":"master","overrides":{"shape":{"visible":false}}}}),
        json!({"op":"create_note","id":"intent","nodeId":"frame","text":"Keep the hierarchy","author":"designer"}),
    ]);
    for index in 0..rectangles {
        ops.push(json!({"op":"create_node","id":format!("r{index:03}"),"kind":"rectangle","parentId":"frame","props":{"name":format!("Tile {index}"),"x":index,"y":10,"width":20,"height":24,"fill":{"tokenId":"ink"},"radius":3}}));
        ops.push(json!({"op":"upsert_token","token":{"id":format!("t{index:03}"),"name":format!("Unused {index}"),"type":"number","value":index}}));
    }
    let result = server
        .batch(&project, 0, "rows-fixture", Value::Array(ops))
        .await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    project
}

#[tokio::test]
async fn rows_round_trip_context_states_instances_notes_and_depth() {
    let server = Server::start().await;
    let project = fixture(&server, 0).await;
    for compact in [false, true] {
        for (instances, depth) in [("reference", 1), ("expand", 1), ("expand", 2)] {
            let mut args = json!({"projectId":project,"frameId":"frame","compact":compact,"instances":instances,"statesMode":"resolved","depth":depth,"properties":["fill","name","text","states"]});
            let expected = context(&server, args.clone()).await;
            let instance = &expected["nodes"][2];
            assert_eq!(instance["id"], "instance");
            assert_eq!(instance["resolvedStates"]["hover"]["fill"], "#123456");
            if instances == "reference" {
                assert_eq!(instance["componentId"], "master");
                assert_eq!(instance["childCount"], 1);
                assert!(instance.get("resolvedProps").is_none());
                assert!(expected["components"]["master"]["nodes"][0].is_object());
            } else if depth == 1 {
                assert_eq!(instance["omittedChildren"], 1);
            } else {
                assert_eq!(expected["nodes"][3]["id"], "instance/shape");
            }
            assert_eq!(expected["notes"][0]["id"], "intent");
            args["nodeFormat"] = json!("rows");
            let mut actual = context(&server, args).await;
            decode_rows(&mut actual);
            assert_eq!(actual, expected);
        }
    }
}

async fn pages(server: &Server, mut args: Value) -> Vec<Value> {
    let mut results = Vec::new();
    loop {
        let page = context(server, args.clone()).await;
        let next = page["nextCursor"].clone();
        results.push(page);
        if next.is_null() {
            return results;
        }
        assert!(results.len() < 200, "unexpected page count");
        args["cursor"] = next;
    }
}

fn items<'a>(pages: &'a [Value], field: &str) -> Vec<&'a Value> {
    pages
        .iter()
        .flat_map(|page| page[field].as_array().into_iter().flatten())
        .collect()
}

#[tokio::test]
async fn rows_count_against_byte_budget_and_cursors_bind_format_and_revision() {
    let server = Server::start().await;
    let project = fixture(&server, 24).await;
    let mut args = json!({"projectId":project,"frameId":"frame","budget":4096,"compact":true});
    let objects = pages(&server, args.clone()).await;
    let object_cursor = objects[0]["nextCursor"].clone();
    args["nodeFormat"] = json!("rows");
    let mut rows = pages(&server, args.clone()).await;
    let rows_cursor = rows[0]["nextCursor"].clone();
    assert!(objects.len() > 1 && rows.len() > 1);
    assert!(items(&rows[..1], "nodes").len() > items(&objects[..1], "nodes").len());
    assert!(rows[0]["components"]["master"]["nodes"][0].is_object());
    assert_eq!(rows[0]["components"], objects[0]["components"]);
    assert!(rows[1..]
        .iter()
        .all(|page| page.get("components").is_none()));
    for page in &mut rows {
        assert!(serde_json::to_vec(page).unwrap().len() <= 4096);
        decode_rows(page);
    }
    for field in ["nodes", "notes", "tokens", "fonts", "styles", "assets"] {
        assert_eq!(items(&rows, field), items(&objects, field), "{field}");
    }
    assert_eq!(items(&rows, "notes")[0]["id"], "intent");
    args["cursor"] = object_cursor;
    assert_eq!(
        server.call("get_design_context", args.clone()).await["error"]["code"],
        "INVALID_CURSOR"
    );
    args["cursor"] = rows_cursor;
    let mut different_format = args.clone();
    different_format
        .as_object_mut()
        .unwrap()
        .remove("nodeFormat");
    assert_eq!(
        server.call("get_design_context", different_format).await["error"]["code"],
        "INVALID_CURSOR"
    );
    let commit = server
        .batch(
            &project,
            1,
            "update",
            json!([{"op":"update_node","id":"text","props":{"text":"Changed"}}]),
        )
        .await;
    assert_eq!(commit["committedRevision"], 2);
    assert_eq!(
        server.call("get_design_context", args).await["error"]["code"],
        "INVALID_CURSOR"
    );
}

#[tokio::test]
async fn node_rows_is_discoverable_only_on_design_context() {
    let server = Server::start().await;
    let list = server.rpc("tools/list", json!({})).await;
    let tools: Vec<_> = list["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|tool| {
            tool["inputSchema"]["properties"]
                .get("nodeFormat")
                .is_some()
        })
        .collect();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "get_design_context");
    assert_eq!(
        tools[0]["inputSchema"]["properties"]["nodeFormat"]["enum"],
        json!(["rows"])
    );
    let project = fixture(&server, 0).await;
    for (tool, args) in [
        (
            "get_design_context",
            json!({"projectId":project,"frameId":"frame","nodeFormat":"rows-v1"}),
        ),
        (
            "get_design_context",
            json!({"projectId":project,"frameId":"frame","nodeFormat":null}),
        ),
        (
            "get_outline",
            json!({"projectId":project,"nodeFormat":"rows"}),
        ),
        (
            "get_subtree",
            json!({"projectId":project,"nodeId":"frame","nodeFormat":"rows"}),
        ),
        (
            "find_nodes",
            json!({"projectId":project,"kinds":["frame"],"nodeFormat":"rows"}),
        ),
        (
            "list_fonts",
            json!({"projectId":project,"nodeFormat":"rows"}),
        ),
    ] {
        assert_eq!(
            server.call(tool, args).await["error"]["code"],
            "INVALID_REQUEST"
        );
    }
}
