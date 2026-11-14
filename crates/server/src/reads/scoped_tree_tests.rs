//! Scoped tree response parity with the frozen full read implementation.
use super::{pagination, scoped_tree_fixture::*, tree, tree_legacy, ReadRequest};
use opencanvas_storage::{read_scope::ScopedSnapshot, SharedSnapshot, Snapshot};
use serde_json::{json, Value};

fn one(s: &Snapshot, tool: &str, args: &Value, scoped: bool) -> Value {
    let req: ReadRequest = serde_json::from_value(args.clone()).unwrap();
    let (scope, offset) = match pagination::scope(s, tool, &req) {
        Ok(v) => v,
        Err(e) => return e.value(),
    };
    let result = if scoped {
        let scoped = ScopedSnapshot::validate(s).unwrap();
        tree::read_scoped(&scoped, tool, req, scope, offset, None)
    } else {
        tree_legacy::read(s, tool, req, scope, offset, None)
    };
    result.unwrap_or_else(|e| e.value())
}

fn parity(s: &Snapshot, tool: &str, mut args: Value) {
    args["projectId"] = json!(s.project_id);
    for _ in 0..100 {
        let expected = one(s, tool, &args, false);
        let actual = one(s, tool, &args, true);
        assert_eq!(
            serde_json::to_vec(&actual).unwrap(),
            serde_json::to_vec(&expected).unwrap(),
            "{tool} {args}\nactual={actual}\nexpected={expected}"
        );
        if actual["truncated"] != true {
            return;
        }
        args["cursor"] = actual["nextCursor"].clone();
    }
    panic!("cursor failed to terminate");
}

#[test]
fn scoped_tree_full_bytes_and_cursors_cover_dependencies_and_metadata() {
    let (_dir, storage, id) = fixture();
    let stored = storage.snapshot(&id).unwrap();
    for s in [&stored, &reordered(&stored)] {
        for tool in ["get_outline", "get_subtree"] {
            for target in [
                "page",
                "frame",
                "placed",
                "placed/nested",
                "placed/nested/copy",
                "absent",
            ] {
                for extra in [
                    json!({}),
                    json!({"limit":2,"depth":1}),
                    json!({"properties":["name","fill"],"budget":2048}),
                ] {
                    let mut args = extra;
                    args["nodeId"] = json!(target);
                    parity(s, tool, args);
                }
            }
        }
        for target in ["frame", "master", "placed", "placed/nested", "absent"] {
            for extra in [
                json!({}),
                json!({"compact":true,"limit":2}),
                json!({"instances":"expand","statesMode":"resolved","notes":"all","limit":3}),
                json!({"compact":true,"instances":"expand","nodeFormat":"rows","theme":"night","depth":1,"properties":["name","fill"],"limit":2}),
                json!({"notes":"none","budget":2048}),
                json!({"theme":"missing"}),
                json!({"theme":"missing","instances":"expand"}),
            ] {
                let mut args = extra;
                args["frameId"] = json!(target);
                parity(s, "get_design_context", args);
            }
        }
        for (tool, args) in [
            ("get_subtree", json!({})),
            ("get_design_context", json!({})),
            (
                "get_design_context",
                json!({"frameId":"frame","nodeId":"frame"}),
            ),
            (
                "get_design_context",
                json!({"frameId":"frame","cursor":"broken","theme":"missing"}),
            ),
            ("get_subtree", json!({"nodeId":"absent","revision":999})),
        ] {
            parity(s, tool, args);
        }
    }
}

fn shared_read(shared: &SharedSnapshot, tool: &str, args: Value) -> Value {
    let req = serde_json::from_value(args).unwrap();
    let (scope, offset) = pagination::scope(&shared.snapshot, tool, &req).unwrap();
    tree::read_shared(shared, tool, req, scope, offset, None).unwrap_or_else(|e| e.value())
}

#[test]
fn shared_tree_cold_warm_and_committed_revisions_stay_isolated() {
    let (_dir, storage, id) = fixture();
    let old = storage.shared_snapshot(&id).unwrap();
    let args = json!({"projectId":id,"frameId":"frame","compact":true});
    let expected = one(&old.snapshot, "get_design_context", &args, false);
    for _ in 0..2 {
        assert_eq!(
            shared_read(&old, "get_design_context", args.clone()),
            expected
        );
    }
    let project = storage.open_project(&id).unwrap();
    project.apply_batch(serde_json::from_value(json!({"projectId":id,"expectedRevision":1,"idempotencyKey":"edit","operations":[{"op":"update_node","id":"copy","props":{"text":"Changed"}}]})).unwrap()).unwrap();
    let new = storage.shared_snapshot(&id).unwrap();
    assert_eq!(
        shared_read(&old, "get_design_context", args.clone()),
        expected
    );
    let changed = shared_read(&new, "get_design_context", args.clone());
    assert_eq!(
        changed,
        one(&new.snapshot, "get_design_context", &args, false)
    );
    assert_ne!(changed, expected);
    assert_eq!(old.snapshot.revision, 1);
    assert_eq!(new.snapshot.revision, 2);
}

#[test]
fn untrusted_backend_keeps_legacy_validation_and_missing_target_behavior() {
    let (_dir, storage, id) = fixture();
    let mut snapshot = storage.snapshot(&id).unwrap();
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "copy")
        .unwrap()
        .props["fontId"] = json!("missing-font");
    let snapshot = std::sync::Arc::new(snapshot);
    let trusted = storage.shared_snapshot(&id).unwrap();
    let untrusted = SharedSnapshot::untrusted(snapshot, trusted.indexes.clone());
    assert!(untrusted.trusted_scoped().unwrap().is_none());
    let mut swapped = storage.shared_snapshot(&id).unwrap();
    swapped.snapshot = untrusted.snapshot.clone();
    assert!(swapped.trusted_scoped().unwrap().is_none());
    for (tool, args) in [
        ("get_outline", json!({"projectId":id,"nodeId":"frame"})),
        ("get_subtree", json!({"projectId":id,"nodeId":"copy"})),
        (
            "get_design_context",
            json!({"projectId":id,"frameId":"frame"}),
        ),
        (
            "get_design_context",
            json!({"projectId":id,"frameId":"absent","theme":"missing"}),
        ),
        ("get_outline", json!({"projectId":id})),
    ] {
        assert_eq!(
            shared_read(&untrusted, tool, args.clone()),
            one(&untrusted.snapshot, tool, &args, false)
        );
        assert_eq!(
            shared_read(&swapped, tool, args.clone()),
            one(&swapped.snapshot, tool, &args, false)
        );
    }
}

#[test]
fn one_design_context_read_builds_scoped_resources_once() {
    let (_dir, storage, id) = fixture();
    let s = storage.snapshot(&id).unwrap();
    let builds = opencanvas_storage::read_scope::resource_builds;
    let before = builds();
    // Resolution plus the default full token resource entries.
    one(
        &s,
        "get_design_context",
        &json!({"projectId":id,"frameId":"frame"}),
        true,
    );
    assert_eq!(builds() - before, 1);
}
