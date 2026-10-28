mod common;
mod links_support;
use common::*;
use links_support::*;
use opencanvas_storage::*;
use serde_json::json;

#[test]
fn links_round_trip_and_empty_means_none() {
    let (_temp, _store, p) = setup();
    let r = screens(&p);
    let links = json!([{"to":"b"}]);
    let out = p
        .apply_batch(request(
            &p,
            r,
            "link",
            vec![update("a-btn", json!({"links":links}))],
        ))
        .unwrap();
    assert_eq!(out.committed_revision, r + 1);
    assert_eq!(out.changed_ids, ["a-btn"]);
    assert_eq!(props(&p, "a-btn")["links"], links);
    assert_eq!(node_links(&props(&p, "a-btn"))[0].action, "navigate");
    p.apply_batch(request(
        &p,
        r + 1,
        "none",
        vec![update("a-btn", json!({"links":[]}))],
    ))
    .unwrap();
    assert_eq!(props(&p, "a-btn")["links"], json!([]));
}

#[test]
fn a_link_may_point_at_a_frame_created_later_in_the_batch() {
    let (_temp, _store, p) = setup();
    let r = screens(&p);
    let ops = vec![
        button("pay", "a", json!([{"to":"checkout"}])),
        frame("checkout", "p1", "Pay"),
    ];
    p.apply_batch(request(&p, r, "forward", ops)).unwrap();
}

#[test]
fn link_shapes_fail_with_the_field_path_op_and_node() {
    let (_temp, _store, p) = setup();
    let r = screens(&p);
    let cases = [
        ("p1", json!([{"to":"b"}]), "props.links", None),
        ("a-btn", json!({"to":"b"}), "links", None),
        (
            "a-btn",
            json!([{"trigger":"click","to":"b"}]),
            "links[0].trigger",
            Some("on"),
        ),
        (
            "a-btn",
            json!([{"destination":"b"}]),
            "links[0].destination",
            Some("to"),
        ),
        ("a-btn", json!([{"on":"tap","to":"b"}]), "links[0].on", None),
        (
            "a-btn",
            json!([{"action":"swap","to":"b"}]),
            "links[0].action",
            None,
        ),
        ("a-btn", json!([{"action":"overlay"}]), "links[0].to", None),
        (
            "a-btn",
            json!([{"action":"back","to":"b"}]),
            "links[0].to",
            None,
        ),
        ("a-btn", json!([{"to":"b"},{"to":"d"}]), "links[1].on", None),
        (
            "a-btn",
            json!([{"on":"hover","to":"d"}]),
            "links[0].action",
            None,
        ),
        ("a-btn", json!(["b"]), "links[0]", None),
        ("a-btn", json!([{"to":"not an id"}]), "links[0].to", None),
    ];
    for (i, (node, links, field, hint)) in cases.into_iter().enumerate() {
        let ops = vec![
            update("a", json!({"name":"Home"})),
            update(node, json!({"links":links})),
        ];
        let e = rejected(&p, r, &format!("shape-{i}"), ops);
        assert_eq!(e.field.as_deref(), Some(field), "case {i}: {e:?}");
        assert_eq!(e.op_index, Some(1), "case {i}");
        assert_eq!(node_id(&e), Some(node), "case {i}");
        assert_eq!(suggestion(&e), hint, "case {i}");
    }
    let ok = json!([{"to":"b"},{"on":"hover","action":"overlay","to":"d"}]);
    p.apply_batch(request(
        &p,
        r,
        "both",
        vec![update("a-btn", json!({"links":ok}))],
    ))
    .unwrap();
}

#[test]
fn link_targets_must_be_root_frames_at_the_end_of_the_batch() {
    let (_temp, _store, p) = setup();
    let r = screens(&p);
    let cases = [
        ("missing", "does not exist", None),
        ("c-btn", "is a rectangle", None),
        ("a-row", "is inside frame 'a'", Some("a")),
        ("Checkout", "does not exist", Some("b")),
    ];
    for (i, (to, message, hint)) in cases.into_iter().enumerate() {
        let ops = vec![update("a-btn", json!({"links":[{"to":to}]}))];
        let e = rejected(&p, r, &format!("target-{i}"), ops);
        assert_eq!(e.field.as_deref(), Some("links[0].to"), "{e:?}");
        assert_eq!(e.op_index, None);
        assert_eq!(node_id(&e), Some("a-btn"));
        assert!(e.message.contains(message), "{}", e.message);
        assert_eq!(suggestion(&e), hint);
    }
    let r = r + 1;
    p.apply_batch(request(
        &p,
        r - 1,
        "link",
        vec![update("a-btn", json!({"links":[{"to":"b"}]}))],
    ))
    .unwrap();
    let nest = Operation::ReparentNode {
        id: "b".into(),
        parent_id: "d".into(),
        index: None,
    };
    let e = rejected(&p, r, "nest", vec![nest]);
    assert_eq!(node_id(&e), Some("a-btn"));
    assert!(e.message.contains("inside frame 'd'"));
}

#[test]
fn unsupported_navigation_keys_suggest_links() {
    let (_temp, _store, p) = setup();
    let r = screens(&p);
    for (i, key) in ["reactions", "onClick", "interactions"]
        .into_iter()
        .enumerate()
    {
        let e = rejected(
            &p,
            r,
            &format!("alias-{i}"),
            vec![update("a-btn", json!({key:[]}))],
        );
        assert_eq!(suggestion(&e), Some("links"), "{key}");
    }
    let e = rejected(
        &p,
        r,
        "start",
        vec![update("a", json!({"startingPoint":"Main"}))],
    );
    assert_eq!(suggestion(&e), Some("flowStart"));
}
