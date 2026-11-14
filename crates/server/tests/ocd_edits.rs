//! `apply_batch` `source` with JSON `operations` in one atomic batch, and the OCD note and
//! sibling-order directives, end to end on temp SQLite storage.
#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

const SEED: &str = "page#p\n  frame#card 200x100 col\n    rect#a 10x10\n    rect#b 10x10\n";

fn batch(project: &str, revision: u64, key: &str, source: &str, ops: Option<Value>) -> Value {
    let mut args = json!({"projectId":project,"expectedRevision":revision,"idempotencyKey":key,
        "source":source});
    if let Some(ops) = ops {
        args["operations"] = ops;
    }
    args
}

async fn revision(s: &Server, p: &str) -> Value {
    s.call("get_outline", json!({"projectId":p})).await["revision"].clone()
}

async fn children(s: &Server, p: &str) -> Vec<String> {
    let read = s
        .call("find_nodes", json!({"projectId":p,"parentId":"card"}))
        .await;
    let nodes = read["matches"].as_array().unwrap();
    nodes
        .iter()
        .map(|n| n["id"].as_str().unwrap().into())
        .collect()
}

#[tokio::test]
async fn source_runs_before_operations_in_one_batch() {
    let s = Server::start().await;
    let p = s.project().await;
    let ops = json!([{"op":"create_node","id":"c","kind":"rectangle","parentId":"card",
        "props":{"width":10,"height":10}},{"op":"reorder_node","id":"c","index":0}]);
    let result = s
        .call("apply_batch", batch(&p, 0, "mixed", SEED, Some(ops)))
        .await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    assert_eq!(children(&s, &p).await, ["c", "a", "b"]);
    // Only the source part generates IDs; none here.
    assert!(result.get("autoIds").is_none(), "{result}");
}

#[tokio::test]
async fn a_failure_in_either_part_rolls_back_both_and_is_located() {
    let s = Server::start().await;
    let p = s.project().await;
    // JSON part fails: opIndex counts within `operations`, no source line.
    let ops = json!([{"op":"reorder_node","id":"a","index":0},
        {"op":"update_node","id":"missing","props":{"name":"x"}}]);
    let failed = s
        .call("apply_batch", batch(&p, 0, "j", SEED, Some(ops)))
        .await;
    let error = &failed["error"];
    assert_eq!(
        (&error["code"], &error["opIndex"]),
        (&json!("VALIDATION_FAILED"), &json!(1)),
        "{failed}"
    );
    assert!(error.get("line").is_none(), "{failed}");
    // Source part fails at storage: its line and column.
    let source = format!("{SEED}    rect 1x1 .missing\n");
    let ops = json!([{"op":"reorder_node","id":"a","index":1}]);
    let failed = s
        .call("apply_batch", batch(&p, 0, "s", &source, Some(ops)))
        .await;
    let error = &failed["error"];
    assert_eq!(
        (&error["line"], &error["column"], &error["opIndex"]),
        (&json!(5), &json!(5), &json!(4)),
        "{failed}"
    );
    assert_eq!(revision(&s, &p).await, 0);
    // The combined batch keeps the 500-operation limit.
    let source = "@del x\n".repeat(400);
    let ops = Value::Array(vec![json!({"op":"delete_subtree","id":"x"}); 101]);
    let failed = s
        .call("apply_batch", batch(&p, 0, "n", &source, Some(ops)))
        .await;
    assert_eq!(failed["error"]["code"], "INVALID_REQUEST", "{failed}");
    assert!(
        failed["error"]["message"].as_str().unwrap().contains("501"),
        "{failed}"
    );
}

#[tokio::test]
async fn note_directives_create_reply_edit_resolve_and_delete() {
    let s = Server::start().await;
    let p = s.project().await;
    let source =
        format!("{SEED}@note card \"Why a column\" author=Ann id=n1\n@note a \"Copy\" id=n2\n");
    let created = s
        .call("apply_batch", batch(&p, 0, "notes", &source, None))
        .await;
    assert_eq!(created["committedRevision"], 1, "{created}");
    let edit = "@reply n1 \"Agreed\" author=Bo\n@note-set n1 \"Why: a column\" node=a\n\
        @resolve n2\n@order b 0\n";
    let edited = s
        .call("apply_batch", batch(&p, 1, "edit", edit, None))
        .await;
    assert_eq!(edited["committedRevision"], 2, "{edited}");
    let read = json!({"projectId":p,"frameId":"card","notes":"all"});
    let notes = s.call("get_design_context", read.clone()).await["notes"].clone();
    assert_eq!(
        notes,
        json!([{"id":"n1","nodeId":"a","text":"Why: a column","author":"Ann",
            "replies":[{"text":"Agreed","author":"Bo"}]},
            {"id":"n2","nodeId":"a","text":"Copy","resolved":true}])
    );
    assert_eq!(children(&s, &p).await, ["b", "a"]);
    let dropped = s
        .call("apply_batch", batch(&p, 2, "drop", "@note-del n2", None))
        .await;
    assert_eq!(dropped["committedRevision"], 3, "{dropped}");
    let notes = s.call("get_design_context", read).await["notes"].clone();
    assert_eq!(notes.as_array().unwrap().len(), 1, "{notes}");
}
