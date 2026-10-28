//! P02a notes over MCP: get_design_context scope/filter/paging, get_changes grouping, conflicts
//! and no prerender for a notes-only commit.
#[allow(dead_code)] // `get` and `wait_job` are unused here.
mod support;
use serde_json::{json, Value};
use std::time::Duration;
use support::{design, Server};
use tokio::time::timeout;

fn note(id: &str, node: &str) -> Value {
    json!({"op":"create_note","id":id,"nodeId":node,"text":format!("Intent {id}"),"author":"designer"})
}
fn ids(response: &Value) -> Vec<&str> {
    response["notes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|n| n["id"].as_str().unwrap())
        .collect()
}
async fn context(s: &Server, p: &str, frame: &str, extra: Value) -> Value {
    let mut args = json!({"projectId":p,"frameId":frame});
    args.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    s.call("get_design_context", args).await
}

#[tokio::test]
async fn design_context_lists_notes_of_ancestors_frame_and_descendants() {
    let s = Server::start().await;
    let p = s.project().await;
    let mut ops = design().as_array().unwrap().clone();
    ops.extend([
        json!({"op":"create_node","id":"rect","kind":"rectangle","parentId":"frame","props":{"width":10,"height":10}}),
        json!({"op":"create_node","id":"other","kind":"frame","parentId":"page","props":{"width":10,"height":10}}),
        json!({"op":"create_node","id":"page2","kind":"page","parentId":null,"props":{"name":"Two"}}),
        json!({"op":"create_node","id":"bare","kind":"frame","parentId":"page2","props":{"width":10,"height":10}}),
        // Created out of tree order: the read orders by node, then by note ID.
        note("n-text", "text"),
        note("n-rect", "rect"),
        note("n-frame", "frame"),
        note("n-page", "page"),
        note("n-other", "other"),
        json!({"op":"update_note","id":"n-rect","resolved":true}),
    ]);
    s.batch(&p, 0, "seed", Value::Array(ops)).await;
    let open = context(&s, &p, "frame", json!({})).await;
    assert_eq!(ids(&open), ["n-page", "n-frame", "n-text"], "{open}");
    assert_eq!(
        open["notes"][0],
        json!({"id":"n-page","nodeId":"page","text":"Intent n-page","author":"designer"})
    );
    let all = context(&s, &p, "frame", json!({"notes":"all"})).await;
    assert_eq!(ids(&all), ["n-page", "n-frame", "n-text", "n-rect"]);
    assert_eq!(all["notes"][3]["resolved"], true);
    let none = context(&s, &p, "frame", json!({"notes":"none"})).await;
    assert!(none.get("notes").is_none());
    let explicit = context(&s, &p, "frame", json!({"notes":"open"})).await;
    assert_eq!(explicit, open);
    // No note in scope: the default read is byte-identical to notes:"none".
    let bare = context(&s, &p, "bare", json!({})).await;
    let bare_none = context(&s, &p, "bare", json!({"notes":"none"})).await;
    assert!(bare.get("notes").is_none());
    assert_eq!(
        serde_json::to_vec(&bare).unwrap(),
        serde_json::to_vec(&bare_none).unwrap()
    );
}

#[tokio::test]
async fn notes_page_at_one_hundred_and_bind_the_cursor_to_the_filter() {
    let s = Server::start().await;
    let p = s.project().await;
    let mut ops = design().as_array().unwrap().clone();
    ops.extend((0..150).map(|i| note(&format!("n{i:03}"), "text")));
    s.batch(&p, 0, "seed", Value::Array(ops)).await;
    let first = context(&s, &p, "frame", json!({})).await;
    assert_eq!(
        (ids(&first).len(), first["truncated"].as_bool()),
        (100, Some(true))
    );
    let cursor = first["nextCursor"].as_str().unwrap();
    let second = context(&s, &p, "frame", json!({"cursor":cursor})).await;
    assert_eq!(ids(&second).len(), 50);
    assert_eq!(ids(&second)[0], "n100");
    assert_eq!(second["truncated"], false);
    for page in [&first, &second] {
        assert!(serde_json::to_vec(page).unwrap().len() <= 65536);
    }
    let other = context(&s, &p, "frame", json!({"cursor":cursor,"notes":"all"})).await;
    assert_eq!(other["error"]["code"], "INVALID_CURSOR", "{other}");
}

#[tokio::test]
async fn changes_group_notes_and_conflicts_name_them() {
    let s = Server::start().await;
    let p = s.project().await;
    s.batch(&p, 0, "seed", design()).await;
    let r = s.batch(&p, 1, "note", json!([note("n1", "frame")])).await;
    assert_eq!(r["changedIds"], json!(["n1"]), "{r}");
    let changes = s
        .call("get_changes", json!({"projectId":p,"sinceRevision":1}))
        .await;
    let changed = changes["changed"].as_object().unwrap();
    assert_eq!(changed.len(), 7, "{changes}");
    assert_eq!(
        (&changed["notes"], &changed["nodes"]),
        (&json!(["n1"]), &json!([]))
    );
    let stale = json!([{"op":"update_node","id":"text","props":{"text":"x"}}]);
    let lost = s.batch(&p, 1, "stale", stale).await;
    assert_eq!(lost["error"]["code"], "REVISION_CONFLICT");
    assert_eq!(lost["error"]["changedSince"]["changedIds"], json!(["n1"]));
    s.batch(&p, 2, "drop", json!([{"op":"delete_note","id":"n1"}]))
        .await;
    let changes = s
        .call("get_changes", json!({"projectId":p,"sinceRevision":1}))
        .await;
    assert_eq!(changes["changed"]["removed"], json!(["n1"]));
    assert_eq!(changes["changed"]["notes"], json!([]));
}

#[tokio::test]
async fn a_notes_only_commit_enqueues_no_prerender() {
    let s = Server::start().await;
    let p = s.project().await;
    let mut events = s.state.events.subscribe();
    s.batch(&p, 0, "seed", design()).await;
    let notes = json!([note("n1", "frame"), {"op":"reply_note","id":"n1","text":"ok"}]);
    assert_eq!(s.batch(&p, 1, "notes", notes).await["committedRevision"], 2);
    let edit = json!([{"op":"update_node","id":"text","props":{"text":"Edited"}}]);
    assert_eq!(s.batch(&p, 2, "edit", edit).await["committedRevision"], 3);
    // Prerenders run in commit order: revision 3 is ready only after any revision 2 job.
    let mut ready = Vec::new();
    while let Ok(Ok(event)) = timeout(Duration::from_secs(45), events.recv()).await {
        if event["event"] == "scene_ready" {
            ready.push(event["revision"].as_u64().unwrap());
            if ready.last() == Some(&3) {
                break;
            }
        }
    }
    assert!(ready.contains(&3), "{ready:?}");
    assert!(!ready.contains(&2), "{ready:?}");
}

#[tokio::test]
async fn note_operations_cost_little_in_tools_list() {
    let s = Server::start().await;
    let tools = s.rpc("tools/list", json!({})).await;
    let bytes = serde_json::to_vec(&tools).unwrap().len();
    // tools/list at night/integration 51f6368, before notes; notes may add at most 2 000 bytes.
    assert!(bytes <= TOOLS_LIST_BASELINE + 2000, "{bytes}");
}
const TOOLS_LIST_BASELINE: usize = 20_513;
