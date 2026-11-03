//! Guide examples must pass the live MCP schema and runtime with an isolated store.
mod authoring_support;
#[allow(dead_code)]
mod support;
use authoring_support::*;
use serde_json::json;
use support::Server;

#[tokio::test]
async fn published_fewshots_discover_reuse_edit_dry_run_and_file() {
    let s = Server::start().await;
    discover(&s).await;
    let text = guidance(&s).await;
    let calls = example(&text, "structured-authoring-calls");
    let p = s.project().await;
    let initial = s.call("get_outline", json!({"projectId":p})).await;
    assert_eq!(initial["revision"], 0);
    assert_eq!(invoke(&s, &p, &calls[0]).await["committedRevision"], 1);
    assert_eq!(invoke(&s, &p, &calls[1]).await["committedRevision"], 2);

    let mut events = s.state.events.subscribe();
    let dry = invoke(&s, &p, &calls[2]).await;
    assert_eq!(dry["dryRun"], true);
    assert_eq!(dry["baseRevision"], 2);
    assert_eq!(dry["diagnostics"], json!([]));
    assert!(dry.get("committedRevision").is_none());
    assert!(matches!(
        events.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    assert_eq!(subtree(&s, &p, "label").await["revision"], 2);
    let mut commit = calls[2].clone();
    commit["arguments"]["dryRun"] = json!(false);
    let committed = invoke(&s, &p, &commit).await;
    assert_eq!(committed["committedRevision"], 3);
    assert_eq!(committed["changedIds"], dry["changedIds"]);

    let ops = example(&text, "structured-authoring-file-operations");
    let inbox = s.root.path().join("test-project/inbox");
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::write(inbox.join("ops.json"), serde_json::to_vec(&ops).unwrap()).unwrap();
    let mut file_call = example(&text, "structured-authoring-file-call");
    let dry = invoke(&s, &p, &file_call).await;
    assert_eq!(dry["dryRun"], true);
    assert_eq!(dry["baseRevision"], 3);
    assert_eq!(subtree(&s, &p, "label").await["revision"], 3);
    file_call["arguments"]["dryRun"] = json!(false);
    let committed = invoke(&s, &p, &file_call).await;
    assert_eq!(committed["committedRevision"], 4);
    assert_eq!(committed["changedIds"], dry["changedIds"]);
    // The response could have been lost: repeat the same body and stable file.
    let replay = invoke(&s, &p, &file_call).await;
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["committedRevision"], 4);
    // Equal operations mean equal content, independent of the MCP input transport.
    let mut inline = arguments(&file_call, &p);
    inline.as_object_mut().unwrap().remove("path");
    inline["operations"] = ops;
    let inline = s.call("apply_batch", inline).await;
    assert_eq!(inline["replayed"], true, "{inline}");
    assert_eq!(inline["committedRevision"], 4);

    let expanded = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"screen","instances":"expand"}),
        )
        .await;
    assert_eq!(expanded["revision"], 4, "{expanded}");
    let nodes = expanded["nodes"].as_array().unwrap();
    for (id, content, fill) in [
        ("first/label", "Saved JSON", "#223344"),
        ("second/label", "Welcome", "#664422"),
    ] {
        let node = nodes.iter().find(|n| n["id"] == id).unwrap();
        assert_eq!(node["resolvedProps"]["text"], content);
        assert_eq!(node["resolvedProps"]["fontSize"], 18);
        assert_eq!(node["resolvedProps"]["fill"], fill);
    }
    assert_eq!(subtree(&s, &p, "label").await["revision"], 4);
}

#[tokio::test]
async fn explicit_conflict_reread_rebuild_and_valid_repetition() {
    let s = Server::start().await;
    let p = s.project().await;
    let text = guidance(&s).await;
    let calls = example(&text, "structured-authoring-calls");
    invoke(&s, &p, &calls[0]).await;
    let repeated = json!({"op":"update_node","id":"label","props":{"text":"Repeated"}});
    let body = json!({"projectId":p,"expectedRevision":1,"idempotencyKey":"repeat-valid",
        "operations":vec![repeated; 500]});
    let committed = s.call("apply_batch", body.clone()).await;
    assert_eq!(committed["committedRevision"], 2, "{committed}");
    let replay = s.call("apply_batch", body).await;
    assert_eq!(replay["replayed"], true);
    let stale = s.call("apply_batch", arguments(&calls[1], &p)).await;
    assert_eq!(stale["error"]["code"], "REVISION_CONFLICT");
    assert_eq!(stale["error"]["changedSince"]["toRevision"], 2);
    let read = subtree(&s, &p, "label").await;
    assert_eq!(read["revision"], 2);
    assert_eq!(read["nodes"][0]["props"]["text"], "Repeated");
    let mut rebuilt = arguments(&calls[1], &p);
    rebuilt["expectedRevision"] = read["revision"].clone();
    rebuilt["idempotencyKey"] = json!("author-rebuilt-after-conflict");
    let committed = s.call("apply_batch", rebuilt).await;
    assert_eq!(committed["committedRevision"], 3, "{committed}");
    assert_eq!(
        subtree(&s, &p, "label").await["nodes"][0]["props"]["text"],
        "Hello again"
    );
}

#[tokio::test]
async fn invented_or_executable_operation_shapes_fail_without_writes() {
    let s = Server::start().await;
    let p = s.project().await;
    let calls = example(&guidance(&s).await, "structured-authoring-calls");
    invoke(&s, &p, &calls[0]).await;
    for (index, (op, code)) in [
        (json!({"op":"update_text","id":"label","text":"Wrong"}), "INVALID_REQUEST"),
        (json!({"op":"update_node","nodeId":"label","props":{"text":"Wrong"}}), "INVALID_REQUEST"),
        (json!({"op":"update_node","id":"label","props":{"content":"Wrong"}}), "VALIDATION_FAILED"),
        (json!({"op":"update_node","id":"label","props":{"text":"Wrong"},"script":"throw 'not executed'"}), "INVALID_REQUEST"),
        (json!({"op":"eval","code":"throw 'not executed'"}), "INVALID_REQUEST"),
    ].into_iter().enumerate() {
        let result = s.batch(&p, 1, &format!("invalid-{index}"), json!([op])).await;
        assert_eq!(result["error"]["code"], code, "{result}");
    }
    let mut executable = arguments(&calls[1], &p);
    executable["code"] = json!("throw 'not executed'");
    let result = s.call("apply_batch", executable).await;
    assert_eq!(result["error"]["code"], "INVALID_REQUEST");
    let unchanged = subtree(&s, &p, "label").await;
    assert_eq!(unchanged["revision"], 1);
    assert_eq!(unchanged["nodes"][0]["props"]["text"], "Hello");
}
