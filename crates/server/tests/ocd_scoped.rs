//! `apply_batch` `source` with `pageIds`: OCD compiles to the same operations as JSON, so the
//! scoped guard must accept, conflict and reject exactly as it does for JSON operations.
#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

/// One `update_node {name}` edit, as JSON operations or as OCD source.
#[derive(Clone, Copy)]
enum Input {
    Json,
    Ocd,
}

impl Input {
    fn rename(self, node: &str, name: &str) -> (&'static str, Value) {
        match self {
            Input::Json => (
                "operations",
                json!([{"op":"update_node","id":node,"props":{"name":name}}]),
            ),
            Input::Ocd => ("source", json!(format!("@set {node} name=\"{name}\""))),
        }
    }
}

async fn project(s: &Server, slug: &str) -> String {
    let created = s
        .call("create_project", json!({"slug":slug,"name":slug}))
        .await;
    let p = created["projectId"].as_str().unwrap().to_owned();
    let seed = s.batch(&p, 0, "seed", json!([
        {"op":"create_node","id":"p1","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"p2","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"f1","kind":"frame","parentId":"p1","props":{"width":40,"height":40}},
        {"op":"create_node","id":"f2","kind":"frame","parentId":"p2","props":{"width":40,"height":40}},
        {"op":"create_node","id":"r1","kind":"rectangle","parentId":"f1","props":{"width":20,"height":20}},
        {"op":"create_node","id":"r2","kind":"rectangle","parentId":"f2","props":{"width":20,"height":20}}
    ])).await;
    assert_eq!(seed["committedRevision"], 1, "{seed}");
    p
}

async fn scoped(
    s: &Server,
    p: &str,
    rev: u64,
    key: &str,
    page: &str,
    edit: (&str, Value),
) -> Value {
    let mut args =
        json!({"projectId":p,"expectedRevision":rev,"idempotencyKey":key,"pageIds":[page]});
    args[edit.0] = edit.1;
    s.call("apply_batch", args).await
}

/// The JSON-comparable part of a result: commit outcome or error code and message.
fn outcome(result: &Value) -> Value {
    match result.get("error") {
        Some(error) => json!({"code":error["code"],"message":error["message"]}),
        None => json!({"revision":result["committedRevision"],"changedIds":result["changedIds"],
            "guard":result["editGuard"]["pageIds"]}),
    }
}

/// Seed, change page p2 at revision 2, then three stale scoped edits based on revision 1.
async fn stale_guard_scenario(s: &Server, input: Input, slug: &str) -> [Value; 3] {
    let p = project(s, slug).await;
    let moved = scoped(s, &p, 1, "p2", "p2", Input::Json.rename("r2", "moved")).await;
    assert_eq!(moved["committedRevision"], 2, "{moved}");
    // p1 is unrelated to the p2 commit: the stale base still commits.
    let unrelated = scoped(s, &p, 1, "unrelated", "p1", input.rename("r1", "a")).await;
    // p2 itself changed after revision 1: conflict.
    let guarded = scoped(s, &p, 1, "guarded", "p2", input.rename("r2", "b")).await;
    let fresh = s.get(&format!("/api/v1/projects/{p}/snapshot")).await["revision"].clone();
    assert_eq!(fresh, 3, "conflict must not commit");
    // r2 lives on p2, outside the declared p1 guard.
    let outside = scoped(s, &p, 3, "outside", "p1", input.rename("r2", "c")).await;
    [unrelated, guarded, outside]
}

#[tokio::test]
async fn ocd_scoped_batches_match_json_guard_outcomes() {
    let s = Server::start().await;
    let json = stale_guard_scenario(&s, Input::Json, "json-scoped").await;
    let ocd = stale_guard_scenario(&s, Input::Ocd, "ocd-scoped").await;

    let [unrelated, guarded, outside] = &ocd;
    assert_eq!(unrelated["committedRevision"], 3, "{unrelated}");
    assert_eq!(unrelated["changedIds"], json!(["r1"]), "{unrelated}");
    assert_eq!(unrelated["editGuard"]["pageIds"], json!(["p1"]));
    assert_eq!(guarded["error"]["code"], "REVISION_CONFLICT", "{guarded}");
    let code = &outside["error"]["code"];
    assert_eq!(code, "SCOPED_OPERATION_UNSUPPORTED", "{outside}");
    for (ocd, json) in ocd.iter().zip(&json) {
        assert_eq!(outcome(ocd), outcome(json), "OCD {ocd} vs JSON {json}");
    }
}
