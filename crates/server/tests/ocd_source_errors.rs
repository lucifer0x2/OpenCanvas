//! `apply_batch` `source` errors over MCP: `@in` misuse fails at the token, errors raised
//! after compilation carry the source position of the failing statement, and both 2 MiB
//! caps fail as source errors. Nothing is committed.
#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

async fn source(s: &Server, project: &str, revision: u64, text: &str) -> Value {
    let args = json!({"projectId":project,"expectedRevision":revision,
        "idempotencyKey":"k","source":text});
    s.call("apply_batch", args).await["error"].clone()
}

fn position(error: &Value) -> (Value, Value) {
    (error["line"].clone(), error["column"].clone())
}

#[tokio::test]
async fn in_rejects_anything_but_one_parent_id() {
    let s = Server::start().await;
    let p = s.project().await;
    let created = s
        .call(
            "apply_batch",
            json!({"projectId":p,"expectedRevision":0,
        "idempotencyKey":"page","source":"page#pg\n  frame#f 100x100"}),
        )
        .await;
    assert_eq!(created["committedRevision"], 1, "{created}");
    for (text, column) in [
        ("@in f row gap=8 fill=#ff0000\n  rect#r 1x1", 7),
        ("@in f \"hello\"", 7),
        ("@in f", 1),
    ] {
        let error = source(&s, &p, 1, text).await;
        assert_eq!(error["code"], "SOURCE_PARSE_FAILED", "{text}: {error}");
        assert_eq!(position(&error), (json!(1), json!(column)), "{text}");
    }
    let outline = s.call("get_outline", json!({"projectId":p})).await;
    assert_eq!(outline["revision"], 1);
}

#[tokio::test]
async fn errors_after_compilation_point_at_their_statement() {
    let s = Server::start().await;
    let p = s.project().await;
    // Published schema: an index over 20000, a note over 2000 characters.
    let error = source(
        &s,
        &p,
        0,
        "page#pg\n  frame#f 10x10\n    rect#r index=30000",
    )
    .await;
    assert_eq!(error["code"], "INVALID_REQUEST", "{error}");
    assert_eq!(position(&error), (json!(3), json!(5)));
    let note = format!("page#pg\n@note pg \"{}\"", "x".repeat(2001));
    let error = source(&s, &p, 0, &note).await;
    assert_eq!(error["code"], "INVALID_REQUEST", "{error}");
    assert_eq!(position(&error), (json!(2), json!(1)));
    // Storage error with opIndex keeps its fields and gains the position.
    let error = source(&s, &p, 0, "page#pg\n  frame#f 10x10 .missing").await;
    assert_eq!(error["code"], "VALIDATION_FAILED", "{error}");
    assert_eq!(error["opIndex"], 1, "{error}");
    assert_eq!(position(&error), (json!(2), json!(3)));
    // The same operations sent as JSON report no source position.
    let ops = json!([{"op":"create_node","id":"pg","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"f","kind":"frame","parentId":"pg",
         "props":{"width":10,"height":10,"styleId":"missing"}}]);
    let json_error = s.batch(&p, 0, "k", ops).await["error"].clone();
    let mut expected = error.clone();
    let object = expected.as_object_mut().unwrap();
    object.remove("line");
    object.remove("column");
    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn both_size_caps_fail_as_source_errors() {
    let s = Server::start().await;
    let p = s.project().await;
    // Under 2 Mi characters, over 2 MiB of UTF-8.
    let text = format!("page#pg\n  text#t \"{}\"", "Đ".repeat(1_100_000));
    let error = source(&s, &p, 0, &text).await;
    assert_eq!(error["code"], "SOURCE_PARSE_FAILED", "{}", error["message"]);
    assert_eq!(error["line"], 2);
    // About 450 KB of source compiles to about 2.4 MB of operations.
    let tokens = vec!["$a"; 150_000].join(",");
    let text = format!("@token a 4\npage#pg\n  frame#f 10x10 p={tokens}");
    let error = source(&s, &p, 0, &text).await;
    assert_eq!(error["code"], "SOURCE_PARSE_FAILED", "{}", error["message"]);
    assert_eq!(
        (&error["field"], position(&error)),
        (&json!("source"), (json!(3), json!(3)))
    );
    assert!(error["message"].as_str().unwrap().contains("2 MiB"));
    let outline = s.call("get_outline", json!({"projectId":p})).await;
    assert_eq!(outline["revision"], 0);
}
