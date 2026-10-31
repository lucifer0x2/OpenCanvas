#[allow(dead_code)]
mod support;
use opencanvas_storage::{allowed_props, NODE_KINDS, PROPS};
use serde_json::{json, Value};
use support::Server;

fn group(id: &str, parent: &str, props: Value) -> Value {
    let mut p = json!({"width":240,"height":80});
    p.as_object_mut()
        .unwrap()
        .extend(props.as_object().unwrap().clone());
    json!({"op":"create_node","id":id,"kind":"group","parentId":parent,"props":p})
}
fn letters(parent: &str) -> Vec<Value> {
    "semanticcopy".chars().enumerate().map(|(i,c)| json!({"op":"create_node","id":format!("{parent}_char{i}"),"kind":"text","parentId":parent,
        "props":{"text":c.to_string(),"x":i*12,"y":10,"width":20,"height":30,"fontId":"font_bundled","fontSize":20,"lineHeight":24}})).collect()
}
fn fragments(value: &Value) -> Vec<&Value> {
    value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"] == "TEXT_FRAGMENTATION_SUSPECTED")
        .collect()
}

#[tokio::test]
async fn capabilities_are_cached_generated_and_unknown_props_still_fail() {
    let s = Server::start().await;
    let resources = s.rpc("resources/list", json!({})).await;
    assert!(resources["ttlMs"].as_u64().unwrap() > 0);
    assert_eq!(resources["cacheScope"], "private");
    assert!(resources["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["uri"] == "opencanvas://capabilities"));
    let read = s
        .rpc("resources/read", json!({"uri":"opencanvas://capabilities"}))
        .await;
    assert!(read["ttlMs"].as_u64().unwrap() > 0);
    assert_eq!(read["contents"][0]["mimeType"], "application/json");
    let cap: Value = serde_json::from_str(read["contents"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(cap["properties"].as_object().unwrap().len(), PROPS.len());
    assert_eq!(cap["catalogFingerprint"].as_str().unwrap().len(), 64);
    for kind in NODE_KINDS {
        assert_eq!(
            cap["keysByKind"][kind.as_str()],
            json!(allowed_props(&kind))
        );
    }
    let p = s.project().await;
    let result = s.batch(&p,0,"bad",json!([{"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"unsupportedTextProperty":true}}])).await;
    assert_eq!(result["error"]["code"], "VALIDATION_FAILED");
    assert_eq!(result["error"]["field"], "props.unsupportedTextProperty");
    assert_eq!(result["error"]["allowedKeys"], cap["keysByKind"]["page"]);
}

#[tokio::test]
async fn fragmentation_scans_effective_instances_before_filters_and_pages_preserves_art() {
    let s = Server::start().await;
    let p = s.project().await;
    let mut ops = vec![
        json!({"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}}),
        json!({"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":500,"height":500}}),
        json!({"op":"create_node","id":"master","kind":"frame","parentId":"page","props":{"width":240,"height":80,"component":{"name":"Copy"}}}),
        group("copy", "frame", json!({})),
        group("wordmark", "frame", json!({"name":"Wordmark"})),
        group("toolbar", "frame", json!({"layout":{"mode":"row"}})),
        group("art", "frame", json!({})),
        json!({"op":"create_node","id":"inst","kind":"instance","parentId":"frame","props":{"componentId":"master"}}),
        json!({"op":"create_note","id":"art-intent","nodeId":"art","text":"Intentional lettering: an expressive wordmark"}),
    ];
    for id in ["copy", "wordmark", "toolbar", "art", "master"] {
        ops.extend(letters(id));
    }
    // A single-character monogram is valid without a note.
    ops.push(json!({"op":"create_node","id":"mono","kind":"text","parentId":"frame","props":{"text":"A","fontId":"font_bundled","fontSize":24,"lineHeight":30}}));
    let result = s.batch(&p, 0, "letters", json!(ops)).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    let args =
        json!({"projectId":p,"frameId":"frame","properties":["name"],"limit":1,"notes":"none"});
    let first = s.call("get_design_context", args.clone()).await;
    let warnings = fragments(&first);
    assert_eq!(warnings.len(), 2, "{first}");
    assert!(warnings.iter().any(|d| d["parentId"] == "copy"));
    let virtuals = warnings.iter().find(|d| d["parentId"] == "inst").unwrap();
    assert!(virtuals["nodeIds"]
        .as_array()
        .unwrap()
        .iter()
        .all(|id| id.as_str().unwrap().starts_with("inst/")));
    assert_eq!(virtuals["nodeIds"].as_array().unwrap().len(), 6);
    let mut next = args.clone();
    next["cursor"] = first["nextCursor"].clone();
    let page = s.call("get_design_context", next).await;
    assert_eq!(page["diagnostics"], first["diagnostics"]);
    let mut expanded = args;
    expanded["instances"] = json!("expand");
    expanded["depth"] = json!(0);
    let expanded = s.call("get_design_context", expanded).await;
    assert_eq!(expanded["diagnostics"], first["diagnostics"]);
    assert!(!warnings
        .iter()
        .any(|d| d.to_string().contains("semanticcopy")));
    let note = s.batch(&p,1,"intent",json!([{"op":"create_note","id":"copy-intent","nodeId":"copy","text":"Intentional lettering: each letter moves separately"}])).await;
    assert_eq!(note["committedRevision"], 2, "{note}");
    let after = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"frame"}),
        )
        .await;
    assert_eq!(fragments(&after).len(), 1);
}

#[tokio::test]
async fn fragmentation_uses_resolved_metrics_and_distinguishes_tracking() {
    let s = Server::start().await;
    let p = s.project().await;
    let mut ops = vec![
        json!({"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}}),
        json!({"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":500,"height":500}}),
        json!({"op":"upsert_token","token":{"id":"size","name":"Body size","type":"dimension","value":20}}),
        json!({"op":"upsert_style","style":{"id":"body","name":"Body","kind":"text","value":{"fontId":"font_bundled","fontSize":{"tokenId":"size"},"lineHeight":24,"letterSpacing":0}}}),
        group("styled", "frame", json!({})),
        group("varied", "frame", json!({})),
    ];
    for parent in ["styled", "varied"] {
        for (i, mut op) in letters(parent).into_iter().enumerate() {
            let props = op["props"].as_object_mut().unwrap();
            for key in ["fontId", "fontSize", "lineHeight"] {
                props.remove(key);
            }
            props.insert("styleId".into(), json!("body"));
            if parent == "varied" {
                props.insert("letterSpacing".into(), json!(i));
            }
            ops.push(op);
        }
    }
    let result = s.batch(&p, 0, "effective", json!(ops)).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    let context = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"frame","properties":["name"],"limit":1}),
        )
        .await;
    let issues = fragments(&context);
    assert_eq!(issues.len(), 1, "{context}");
    assert_eq!(issues[0]["parentId"], "styled");
    assert_eq!(issues[0]["fragmentCount"], 12);
}
