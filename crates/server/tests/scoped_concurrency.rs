#[allow(dead_code)]
mod postgres_backend_support;
#[allow(dead_code)]
mod support;
use serde_json::{json, Value};

trait Mcp {
    async fn call(&self, name: &str, args: Value) -> Value;
    async fn get(&self, path: &str) -> Value;
}
impl Mcp for support::Server {
    async fn call(&self, name: &str, args: Value) -> Value {
        self.call(name, args).await
    }
    async fn get(&self, path: &str) -> Value {
        self.get(path).await
    }
}
impl Mcp for postgres_backend_support::Server {
    async fn call(&self, name: &str, args: Value) -> Value {
        self.call(name, args).await
    }
    async fn get(&self, path: &str) -> Value {
        self.get(path).await
    }
}
fn request(project: &str, revision: u64, key: &str, page: &str, node: &str) -> Value {
    json!({"projectId":project,"expectedRevision":revision,"pageIds":[page],"idempotencyKey":key,
        "operations":[{"op":"update_node","id":node,"props":{"name":key}}]})
}
async fn contract(s: &impl Mcp) {
    let slug = format!("scoped-{}", uuid::Uuid::new_v4().simple());
    let p = s
        .call("create_project", json!({"slug":slug,"name":"Scoped MCP"}))
        .await["projectId"]
        .as_str()
        .unwrap()
        .to_owned();
    let seed=s.call("apply_batch",json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"seed","operations":[
        {"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":"#123456"}},
        {"op":"create_node","id":"p1","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"p2","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"f1","kind":"frame","parentId":"p1","props":{"width":40,"height":40}},
        {"op":"create_node","id":"f2","kind":"frame","parentId":"p2","props":{"width":40,"height":40}},
        {"op":"create_node","id":"r1","kind":"rectangle","parentId":"f1","props":{"width":20,"height":20,"fill":{"tokenId":"ink"}}},
        {"op":"create_node","id":"r2","kind":"rectangle","parentId":"f2","props":{"width":20,"height":20}}
    ]})).await;
    assert_eq!(seed["committedRevision"], 1, "{seed}");
    let context = s
        .call("get_edit_context", json!({"projectId":p,"pageIds":["p1"]}))
        .await;
    assert_eq!(context["expectedRevision"], 1, "{context}");
    assert!(context["dependencies"]
        .as_array()
        .unwrap()
        .contains(&json!("token:ink")));
    assert!(context.get("snapshot").is_none());
    let a = request(&p, 1, "a", "p1", "r1");
    let b = request(&p, 1, "b", "p2", "r2");
    let (ra, rb) = tokio::join!(s.call("apply_batch", a.clone()), s.call("apply_batch", b));
    let mut revisions = vec![
        ra["committedRevision"].as_u64().unwrap(),
        rb["committedRevision"].as_u64().unwrap(),
    ];
    revisions.sort();
    assert_eq!(revisions, [2, 3]);
    assert_eq!(ra["editGuard"]["expectedRevision"], ra["committedRevision"]);
    assert_eq!(s.call("apply_batch", a).await["replayed"], true);
    let stale = s
        .call("apply_batch", request(&p, 1, "stale", "p1", "r1"))
        .await;
    assert_eq!(stale["error"]["code"], "REVISION_CONFLICT", "{stale}");
    let updated=s.call("apply_batch",json!({"projectId":p,"expectedRevision":3,"idempotencyKey":"dep","operations":[
        {"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":"#abcdef"}}
    ]})).await;
    assert_eq!(updated["committedRevision"], 4);
    assert_eq!(
        s.call("apply_batch", request(&p, 3, "dep-stale", "p1", "r1"))
            .await["error"]["code"],
        "REVISION_CONFLICT"
    );
    let mut preview = request(&p, 3, "preview", "p2", "r2");
    preview["dryRun"] = json!(true);
    assert_eq!(s.call("apply_batch", preview).await["baseRevision"], 4);
    let changes = s
        .call(
            "get_changes",
            json!({"projectId":p,"sinceRevision":0,"includeOperations":true}),
        )
        .await;
    assert!(!changes.to_string().contains("scopeVersion"));
    assert_eq!(
        s.get(&format!("/api/v1/projects/{p}/snapshot")).await["revision"],
        4
    );
}
#[tokio::test]
async fn sqlite_real_mcp_scoped_guards_and_legacy_compatibility() {
    contract(&support::Server::start().await).await;
    notes_and_pages(&support::Server::start().await).await;
}
#[tokio::test]
#[ignore = "fresh disposable PostgreSQL only"]
async fn postgres_real_mcp_scoped_guards_and_legacy_compatibility() {
    contract(&postgres_backend_support::Server::start().await).await;
    notes_and_pages(&postgres_backend_support::Server::start().await).await;
}
#[tokio::test]
async fn sqlite_scoped_file_preview_replay_and_original_schema_validation() {
    let s = support::Server::start().await;
    let p = s.project().await;
    assert_eq!(
        s.batch(&p, 0, "seed", support::design()).await["committedRevision"],
        1
    );
    let path = s.root.path().join("test-project/inbox/scoped.json");
    std::fs::write(
        &path,
        json!([{"op":"update_node","id":"text","props":{"text":"Scoped file"}}]).to_string(),
    )
    .unwrap();
    let mut args = json!({"projectId":p,"expectedRevision":1,"pageIds":["page"],"idempotencyKey":"file","path":"scoped.json","dryRun":true});
    assert_eq!(
        s.call("apply_batch_file", args.clone()).await["baseRevision"],
        1
    );
    args["dryRun"] = json!(false);
    assert_eq!(
        s.call("apply_batch_file", args.clone()).await["committedRevision"],
        2
    );
    assert_eq!(s.call("apply_batch_file", args).await["replayed"], true);
}
/// Parallel sessions: a note on one page and a new page do not invalidate another
/// page's guard; a note on the guarded page does.
async fn notes_and_pages(s: &impl Mcp) {
    let slug = format!("scoped-notes-{}", uuid::Uuid::new_v4().simple());
    let created = s
        .call("create_project", json!({"slug":slug,"name":"Notes"}))
        .await;
    let p = created["projectId"].as_str().unwrap().to_owned();
    let seed=s.call("apply_batch",json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"seed","operations":[
        {"op":"create_node","id":"p1","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"p2","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"f1","kind":"frame","parentId":"p1","props":{"width":40,"height":40}},
        {"op":"create_node","id":"f2","kind":"frame","parentId":"p2","props":{"width":40,"height":40}}
    ]})).await;
    assert_eq!(seed["committedRevision"], 1, "{seed}");
    let batch = |key: &str, page: &str, operations: Value| json!({"projectId":p,"expectedRevision":1,"pageIds":[page],"idempotencyKey":key,"operations":operations});
    let note = batch(
        "note",
        "p1",
        json!([{"op":"create_note","id":"n1","nodeId":"f1","text":"Why"}]),
    );
    assert_eq!(s.call("apply_batch", note).await["committedRevision"], 2);
    let page = batch(
        "page",
        "p3",
        json!([{"op":"create_node","id":"p3","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"f3","kind":"frame","parentId":"p3","props":{"width":40,"height":40}}]),
    );
    let page = s.call("apply_batch", page).await;
    assert_eq!(page["editGuard"]["pageIds"], json!(["p3"]), "{page}");
    let other = batch(
        "other",
        "p2",
        json!([{"op":"update_node","id":"f2","props":{"name":"B"}}]),
    );
    assert_eq!(s.call("apply_batch", other).await["committedRevision"], 4);
    let stale = batch(
        "stale",
        "p1",
        json!([{"op":"update_node","id":"f1","props":{"name":"A"}}]),
    );
    let stale = s.call("apply_batch", stale).await;
    assert_eq!(stale["error"]["code"], "REVISION_CONFLICT", "{stale}");
}
