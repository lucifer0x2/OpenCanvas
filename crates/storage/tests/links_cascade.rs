mod bundle_support;
mod common;
mod links_support;
use bundle_support::*;
use common::*;
use links_support::*;
use opencanvas_storage::*;
use serde_json::json;

/// a-btn links to b (click) and overlay d (hover); c-btn on page 2 links only to b.
fn linked(p: &Project) -> u64 {
    let r = screens(p);
    let ops = vec![
        update(
            "a-btn",
            json!({"links":[{"to":"b"},{"on":"hover","action":"overlay","to":"d"}]}),
        ),
        update("c-btn", json!({"links":[{"to":"b"}]})),
    ];
    p.apply_batch(request(p, r, "linked", ops)).unwrap();
    r + 1
}

#[test]
fn deleting_a_target_prunes_links_to_it_and_reports_the_sources() {
    let (_temp, _store, p) = setup();
    let r = linked(&p);
    let out = p
        .apply_batch(request(&p, r, "drop-b", vec![delete("b")]))
        .unwrap();
    assert_eq!(out.changed_ids, ["a-btn", "b", "c-btn"]);
    assert_eq!(
        props(&p, "a-btn")["links"],
        json!([{"on":"hover","action":"overlay","to":"d"}])
    );
    assert!(props(&p, "c-btn").get("links").is_none());
    let changes = p.commits_after(r, 10).unwrap();
    assert_eq!(changes.len(), 1);
}

#[test]
fn recreating_a_deleted_target_in_the_same_batch_keeps_links() {
    let (_temp, _store, p) = setup();
    let r = linked(&p);
    let ops = vec![delete("b"), frame("b", "p1", "Checkout again")];
    let out = p.apply_batch(request(&p, r, "rebuild", ops)).unwrap();
    // Sibling sort keys move ("d"); the link sources do not change.
    assert!(out.changed_ids.contains(&"b".to_string()));
    assert!(!out.changed_ids.iter().any(|id| id.ends_with("-btn")));
    assert_eq!(props(&p, "c-btn")["links"], json!([{"to":"b"}]));
}

#[test]
fn flow_start_marks_one_root_frame_per_name() {
    let (_temp, _store, p) = setup();
    let r = screens(&p);
    p.apply_batch(request(
        &p,
        r,
        "main",
        vec![update("a", json!({"flowStart":"Main"}))],
    ))
    .unwrap();
    let r = r + 1;
    assert_eq!(flow_start(&props(&p, "a")), Some("Main"));
    let cases = [
        ("a-row", json!("Side"), "flowStart"),
        ("c-btn", json!("Side"), "props.flowStart"),
        ("b", json!(""), "flowStart"),
        ("b", json!("x".repeat(65)), "flowStart"),
    ];
    for (i, (node, value, field)) in cases.into_iter().enumerate() {
        let ops = vec![update(node, json!({"flowStart":value}))];
        let e = rejected(&p, r, &format!("start-{i}"), ops);
        assert_eq!(e.field.as_deref(), Some(field), "case {i}: {e:?}");
        assert_eq!(node_id(&e), Some(node));
    }
    let e = rejected(&p, r, "dup", vec![update("c", json!({"flowStart":"Main"}))]);
    assert_eq!(e.field.as_deref(), Some("flowStart"));
    assert_eq!(node_id(&e), Some("c"));
    assert!(e.message.contains("'a'"), "{}", e.message);
    let nest = Operation::ReparentNode {
        id: "a".into(),
        parent_id: "d".into(),
        index: None,
    };
    let e = rejected(&p, r, "nest", vec![nest]);
    assert_eq!(e.field.as_deref(), Some("flowStart"));
}

#[test]
fn bundles_keep_links_and_reject_dangling_ones() {
    let (temp, store, p) = setup();
    linked(&p);
    let bundle = p.export_bundle().unwrap();
    let copy = store.import_bundle("copy", &bundle.bytes).unwrap();
    let original = props(&p, "a-btn")["links"].clone();
    assert_eq!(
        serde_json::to_vec(&props(&copy, "a-btn")["links"]).unwrap(),
        serde_json::to_vec(&original).unwrap()
    );
    let mut files = unpack(&bundle.bytes);
    edit_database(
        &mut files,
        "UPDATE nodes SET props_json=json_set(props_json,'$.links[0].to','gone') WHERE id='c-btn'",
    );
    reject(&store, temp.path(), &pack(files));
}
