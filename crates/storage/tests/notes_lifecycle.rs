//! P02a notes: cascade on delete_subtree, durability across reopen, bundle round trips.
mod bundle_support;
mod common;
mod notes_support;
use bundle_support::*;
use common::*;
use notes_support::*;
use opencanvas_storage::*;
use serde_json::json;

#[test]
fn deleting_a_subtree_removes_its_notes_only() {
    let (_temp, _store, p) = setup();
    let mut ops = initial();
    ops.push(create(
        "other",
        NodeKind::Frame,
        Some("page"),
        json!({"width":10,"height":10}),
    ));
    ops.extend([
        note("on-page", "page", "p"),
        note("on-frame", "frame", "f"),
        note("on-rect", "rect", "r"),
        note("on-other", "other", "o"),
    ]);
    p.apply_batch(request(&p, 0, "initial", ops)).unwrap();
    let del = vec![Operation::DeleteSubtree { id: "frame".into() }];
    let r = p.apply_batch(request(&p, 1, "delete", del)).unwrap();
    for id in ["on-frame", "on-rect", "frame", "rect", "text"] {
        assert!(r.changed_ids.contains(&id.to_string()), "{id}");
    }
    let left: Vec<_> = p
        .snapshot()
        .unwrap()
        .notes
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(left, ["on-other", "on-page"]);
}

#[test]
fn notes_survive_reopen_and_bundle_round_trips() {
    let (temp, store, p) = setup();
    let mut ops = initial();
    ops.extend([note("a", "frame", "Tiếng Việt — ý đồ"), reply("a", "ok")]);
    ops.push(update("a", Some(true)));
    p.apply_batch(request(&p, 0, "initial", ops)).unwrap();
    let before = notes_json(&p);
    let bundle = p.export_bundle().unwrap();
    drop(p);
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    assert_eq!(notes_json(&store.open_project("fixture").unwrap()), before);
    let imported = store.import_bundle("copy", &bundle.bytes).unwrap();
    assert_eq!(notes_json(&imported), before);
    let destination = tempfile::tempdir().unwrap();
    let restored = Store::open(destination.path())
        .unwrap()
        .restore_bundle("restored", &bundle.bytes)
        .unwrap();
    assert_eq!(notes_json(&restored), before);
    let original = unpack(&bundle.bytes);
    for sql in [
        "UPDATE notes SET data_json=json_set(data_json,'$.nodeId','missing')",
        "UPDATE notes SET data_json=json_set(data_json,'$.id','different')",
        "UPDATE notes SET data_json=json_set(data_json,'$.text','')",
    ] {
        let mut files = original.clone();
        edit_database(&mut files, sql);
        reject(&store, temp.path(), &pack(files));
    }
}
