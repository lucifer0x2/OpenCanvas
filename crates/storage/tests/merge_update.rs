//! `update_node` with `merge:true` (a JSON merge patch of props) through the backend trait
//! object, which every backend shares (PostgreSQL validates the same candidate), strict and
//! scoped; without `merge` each top-level key still replaces.
mod backend_contract_support;
use backend_contract_support::{request, setup};
use opencanvas_storage::*;
use serde_json::{json, Value};

fn ops(value: Value) -> Vec<Operation> {
    serde_json::from_value(value).unwrap()
}

fn props(storage: &Storage, project: &str, id: &str) -> Value {
    let snapshot = storage.open_project(project).unwrap().snapshot().unwrap();
    snapshot
        .nodes
        .iter()
        .find(|n| n.id == id)
        .unwrap()
        .props
        .clone()
}

fn seed() -> Vec<Operation> {
    ops(json!([
        {"op":"create_node","id":"p","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"input","kind":"frame","parentId":"p","props":{"width":200,
            "height":40,"component":{"name":"Input"},
            "stroke":{"color":"#E5E7EB","width":1,"align":"inside"},
            "layout":{"mode":"row","padding":[0,12],"gap":4,"align":"center"}}},
        {"op":"create_node","id":"ph","kind":"rectangle","parentId":"input","props":{"width":10,"height":10}},
        {"op":"create_node","id":"icon","kind":"rectangle","parentId":"input","props":{"width":10,"height":10}},
        {"op":"create_node","id":"use","kind":"instance","parentId":"p","props":{"componentId":"input",
            "overrides":{"ph":{"fill":"#111111","visible":true},"icon":{"fill":"#222222"}}}}
    ]))
}

#[test]
fn merge_patches_layout_stroke_and_one_override_and_null_deletes() {
    let (_root, backend, storage) = setup();
    let p = storage.create_project("merge", "Merge").unwrap();
    let id = p.id().to_owned();
    backend
        .apply_batch(&id, request(&id, 0, "seed", seed()))
        .unwrap();
    let patch = ops(json!([
        {"op":"update_node","id":"input","merge":true,"props":{"layout":{"gap":8,"align":null},
            "stroke":{"width":2}}},
        {"op":"update_node","id":"use","merge":true,"props":{"overrides":{"ph":{"fill":{"type":"linear",
            "angle":90,"stops":[[0,"#000000"],[1,"#ffffff"]]},"visible":null}}}}
    ]));
    let result = backend
        .apply_batch(&id, request(&id, 1, "patch", patch.clone()))
        .unwrap();
    assert_eq!(result.committed_revision, 2);
    let input = props(&storage, &id, "input");
    assert_eq!(
        input["layout"],
        json!({"mode":"row","padding":[0,12],"gap":8})
    );
    assert_eq!(
        input["stroke"],
        json!({"color":"#E5E7EB","width":2,"align":"inside"})
    );
    let overrides = &props(&storage, &id, "use")["overrides"];
    assert_eq!(overrides["icon"], json!({"fill":"#222222"}));
    assert_eq!(overrides["ph"]["fill"]["type"], "linear");
    assert!(overrides["ph"].get("visible").is_none(), "{overrides}");
    // The same body replays; the stored request keeps `merge`.
    let replay = backend
        .apply_batch(&id, request(&id, 1, "patch", patch))
        .unwrap();
    assert!(replay.replayed);
    // A reference replaces a gradient instead of mixing keys with it.
    let token = ops(
        json!([{"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":"#000000"}},
        {"op":"update_node","id":"use","merge":true,"props":{"overrides":{"ph":{"fill":{"tokenId":"ink"}}}}}]),
    );
    backend
        .apply_batch(&id, request(&id, 2, "token", token))
        .unwrap();
    let ph = &props(&storage, &id, "use")["overrides"]["ph"];
    assert_eq!(ph, &json!({"fill":{"tokenId":"ink"}}));
}

#[test]
fn without_merge_object_props_replace_and_merge_is_not_serialized() {
    let (_root, backend, storage) = setup();
    let p = storage.create_project("replace", "Replace").unwrap();
    let id = p.id().to_owned();
    backend
        .apply_batch(&id, request(&id, 0, "seed", seed()))
        .unwrap();
    let replace = ops(
        json!([{"op":"update_node","id":"input","props":{"layout":{"mode":"column"}}},
        {"op":"update_node","id":"use","props":{"overrides":{"icon":{"visible":false}}}}]),
    );
    let serialized = serde_json::to_value(&replace[0]).unwrap();
    assert!(serialized.get("merge").is_none(), "{serialized}");
    backend
        .apply_batch(&id, request(&id, 1, "replace", replace))
        .unwrap();
    assert_eq!(
        props(&storage, &id, "input")["layout"],
        json!({"mode":"column"})
    );
    assert_eq!(
        props(&storage, &id, "use")["overrides"],
        json!({"icon":{"visible":false}})
    );
    // A merge that leaves an invalid prop fails like any update, atomically.
    let invalid = ops(json!([{"op":"update_node","id":"input","merge":true,
        "props":{"layout":{"mode":null,"gap":2}}}]));
    let error = backend
        .apply_batch(&id, request(&id, 2, "bad", invalid))
        .unwrap_err();
    assert_eq!(error.op_index, Some(0), "{error:?}");
    assert_eq!(p.snapshot().unwrap().revision, 2);
}

#[test]
fn scoped_batches_merge_too() {
    let (_root, backend, storage) = setup();
    let p = storage.create_project("scoped", "Scoped").unwrap();
    let id = p.id().to_owned();
    backend
        .apply_batch(&id, request(&id, 0, "seed", seed()))
        .unwrap();
    let scoped: ScopedBatchRequest = serde_json::from_value(json!({"projectId":id,
        "expectedRevision":1,"idempotencyKey":"scoped","pageIds":["p"],
        "operations":[{"op":"update_node","id":"input","merge":true,"props":{"layout":{"gap":12}}}]}))
    .unwrap();
    p.apply_scoped_batch(scoped).unwrap();
    let layout = &props(&storage, &id, "input")["layout"];
    assert_eq!(
        layout,
        &json!({"mode":"row","padding":[0,12],"gap":12,"align":"center"})
    );
}
