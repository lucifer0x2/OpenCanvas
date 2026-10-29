mod instances_support;
#[allow(dead_code)]
mod support;
use instances_support::*;
use serde_json::json;
use support::Server;

#[tokio::test]
async fn instances_mcp_ops_pixels_master_invalidation_and_virtual_reads() {
    let s = Server::start().await;
    let project = fixture(&s).await;
    let (instances, manual) = operations();
    let (after, before) = (
        serde_json::to_vec(&instances).unwrap().len(),
        serde_json::to_vec(&manual).unwrap().len(),
    );
    println!(
        "P24 ops {}->{}, bytes {before}->{after}, ratio {:.4}",
        manual.len(),
        instances.len(),
        after as f64 / before as f64
    );
    assert!(after * 100 <= before * 30);
    let instance = render(&s, &project, "frame", 1).await;
    let manual = render(&s, &project, "manual", 1).await;
    assert_eq!(
        png(&s, &instance).await,
        png(&s, &manual).await,
        "PNG must be pixel-identical"
    );
    for i in 0..20 {
        assert_eq!(
            bounds(&instance, &format!("row{i}/cmpRow-q")),
            bounds(&manual, &format!("manual{i}-q"))
        );
    }
    let inspected = s.call("inspect_layout", json!({"projectId":project,"jobId":instance["jobId"],"sceneKey":instance["sceneKey"],"nodeIds":["row3/cmpRow-q"]})).await;
    assert_eq!(inspected["nodes"][0]["id"], "row3/cmpRow-q");
    assert_eq!(
        inspected["nodes"][0]["bounds"],
        *bounds(&instance, "row3/cmpRow-q")
    );
    let changed = s.batch(&project, 1, "master", json!([{"op":"update_node","id":"cmpRow","props":{"layout":{"mode":"row","padding":[8,24],"gap":12}}}])).await;
    assert_eq!(changed["committedRevision"], 2, "{changed}");
    let updated = render(&s, &project, "frame", 2).await;
    assert_ne!(updated["sceneKey"], instance["sceneKey"]);
    assert_ne!(png(&s, &instance).await, png(&s, &updated).await);
    for i in 0..20 {
        let id = format!("row{i}/cmpRow-q");
        assert_eq!(
            bounds(&updated, &id)["x"].as_f64().unwrap()
                - bounds(&instance, &id)["x"].as_f64().unwrap(),
            12.0
        );
    }
    let export = s
        .call(
            "export_nodes",
            json!({"projectId":project,"nodeIds":["row3","row3/cmpRow-q"],"waitMs":10000}),
        )
        .await;
    for result in export["exports"].as_array().unwrap() {
        assert_eq!(result["status"], "ready", "{result}");
    }
    println!(
        "P24 pixels identical; all 20 virtual cell x changed +12; export instance/child ready"
    );
}

#[tokio::test]
async fn instances_mcp_context_reference_expand_find_and_bounds() {
    let s = Server::start().await;
    let project = fixture(&s).await;
    let args = json!({"projectId":project,"frameId":"frame","limit":200});
    let reference = context(&s, args.clone()).await;
    let mut expanded_args = args.clone();
    expanded_args["instances"] = json!("expand");
    let expanded = context(&s, expanded_args.clone()).await;
    assert_eq!(reference["components"].as_object().unwrap().len(), 1);
    assert_eq!(reference["nodes"].as_array().unwrap().len(), 21);
    assert_eq!(expanded["nodes"].as_array().unwrap().len(), 101);
    let (r, e) = (
        serde_json::to_vec(&reference).unwrap().len(),
        serde_json::to_vec(&expanded).unwrap().len(),
    );
    println!(
        "P24 context reference {r} vs expand {e}, ratio {:.4}",
        r as f64 / e as f64
    );
    assert!(r * 2 <= e);
    let found = s.call("find_nodes", json!({"projectId":project,"frameId":"frame","kinds":["instance"],"properties":["componentId"]})).await;
    assert_eq!(found["totalMatches"], 20, "{found}");
    let found = s
        .call(
            "find_nodes",
            json!({"projectId":project,"frameId":"frame","textContains":"t3"}),
        )
        .await;
    assert_eq!(found["matches"][0]["id"], "row3/cmpRow-q");
    let outline = s
        .call("get_outline", json!({"projectId":project,"nodeId":"row3"}))
        .await;
    assert_eq!(outline["nodes"][0]["componentId"], "cmpRow");
    assert_eq!(outline["nodes"][0]["childCount"], 4);
    let subtree = s
        .call(
            "get_subtree",
            json!({"projectId":project,"nodeId":"row3/cmpRow-q"}),
        )
        .await;
    assert_eq!(subtree["nodes"][0]["props"]["text"], "select * from t3");
    let mut args = args;
    args["viewports"] = json!([{"width":800,"height":1000}]);
    let ready = context(&s, args.clone()).await;
    assert_eq!(ready["computed"]["status"], "ready", "{ready}");
    let instance = ready["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "row3")
        .unwrap();
    assert!(instance["computed"]["children"]["row3/cmpRow-q"]["lines"][0].is_array());
    let mut args = args;
    args["instances"] = json!("expand");
    let expanded = context(&s, args).await;
    let child = expanded["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "row3/cmpRow-q")
        .unwrap();
    assert_eq!(
        child["computedBounds"],
        instance["computed"]["children"]["row3/cmpRow-q"]["bounds"]
    );
    let mut paged = json!({"projectId":project,"frameId":"frame","limit":2});
    let first = context(&s, paged.clone()).await;
    paged["cursor"] = first["nextCursor"].clone();
    let second = context(&s, paged).await;
    assert!(first["components"].is_object());
    assert!(second.get("components").is_none());
}

#[tokio::test]
async fn instances_mcp_errors_are_structured_and_atomic() {
    let s = Server::start().await;
    let project = fixture(&s).await;
    for (i, (operation, expected)) in [
        (
            json!({"op":"update_node","id":"row3/cmpRow-q","props":{"text":"bad"}}),
            "INSTANCE_CHILD_READONLY",
        ),
        (
            json!({"op":"reparent_node","id":"row3","parentId":"row3/cmpRow-q"}),
            "INSTANCE_CHILD_READONLY",
        ),
        (
            json!({"op":"delete_subtree","id":"cmpRow"}),
            "COMPONENT_IN_USE",
        ),
        (
            create(
                "cycle",
                "instance",
                Some("cmpRow"),
                json!({"componentId":"cmpRow"}),
            ),
            "INSTANCE_CYCLE",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let result = s
            .batch(&project, 1, &format!("fail{i}"), json!([operation]))
            .await;
        assert_eq!(result["error"]["code"], expected, "{result}");
        if expected == "COMPONENT_IN_USE" {
            assert_eq!(result["error"]["count"], 20);
        }
    }
    let result = s.batch(&project, 1, "key", json!([{"op":"update_node","id":"row3","props":{"overrides":{"cmpRow-q":{"layout":{}}}}}])).await;
    assert_eq!(result["error"]["field"], "props.overrides.cmpRow-q.layout");
    assert!(result["error"]["allowedKeys"]
        .as_array()
        .unwrap()
        .contains(&json!("text")));
    let snapshot = s.get(&format!("/api/v1/projects/{project}/snapshot")).await;
    assert_eq!(snapshot["revision"], 1);
}
