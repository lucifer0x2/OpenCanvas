//! Trusted scoped MCP reads over rows that bypassed the current validator.
use super::{find, pagination, scoped_tree_fixture::corrupt, tree, ReadRequest};
use crate::error::Result;
use opencanvas_storage::SharedSnapshot;
use serde_json::{json, Value};

fn read(shared: &SharedSnapshot, tool: &str, mut args: Value) -> Result<Value> {
    args["projectId"] = json!(shared.snapshot.project_id);
    let req: ReadRequest = serde_json::from_value(args).unwrap();
    if tool == "find_nodes" {
        return find::read_shared(shared, req);
    }
    let (scope, offset) = pagination::scope(&shared.snapshot, tool, &req)?;
    tree::read_shared(shared, tool, req, scope, offset, None)
}

fn calls() -> [(&'static str, Value); 6] {
    [
        ("get_design_context", json!({"frameId":"frame"})),
        ("get_outline", json!({"nodeId":"frame"})),
        ("get_outline", json!({"nodeId":"page"})),
        ("get_subtree", json!({"nodeId":"frame"})),
        ("find_nodes", json!({"frameId":"frame","kinds":["frame"]})),
        ("find_nodes", json!({"nameContains":"a"})),
    ]
}

#[test]
fn placed_instance_with_missing_master_fails_every_trusted_read() {
    let (_dir, storage, id) = corrupt("placed", r#"{"componentId":"missing"}"#);
    let shared = storage.shared_snapshot(&id).unwrap();
    assert!(shared.trusted_scoped().unwrap().is_some());
    for (tool, args) in calls() {
        let error = read(&shared, tool, args.clone()).err();
        let error = error.unwrap_or_else(|| panic!("{tool} {args} succeeded"));
        assert_eq!(
            error.value()["error"]["field"],
            "props.componentId",
            "{tool} {args}"
        );
    }
    // The unrelated library page is still served by bounded trusted reads.
    read(&shared, "get_outline", json!({"nodeId":"library"})).unwrap();
}

#[test]
fn scalar_authored_props_fail_resolution_and_never_panic_elsewhere() {
    let (_dir, storage, id) = corrupt("outgoing", "5");
    let shared = storage.shared_snapshot(&id).unwrap();
    let error = read(&shared, "get_design_context", json!({"frameId":"frame"})).unwrap_err();
    assert_eq!(error.value()["error"]["field"], "props");
    // Authored-only reads do not resolve properties; they must still return.
    for (tool, args) in calls().into_iter().skip(1) {
        let _ = read(&shared, tool, args);
    }
}
