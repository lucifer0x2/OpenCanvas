use super::*;

fn node(id: &str, parent: Option<&str>, kind: NodeKind, key: i64, props: Value) -> Node {
    Node {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        kind,
        sort_key: key,
        props,
    }
}

fn snapshot() -> Snapshot {
    serde_json::from_value(json!({
        "schemaVersion":1,"projectId":"p","slug":"p","name":"P","revision":7,
        "nodes":[],"tokens":[],"styles":[],"assets":[],"fonts":[],
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"retain"}
    }))
    .unwrap()
}

fn query() -> ScenesQuery {
    ScenesQuery {
        page_id: "page".into(),
        revision: 7,
        offset: 0,
        limit: 4,
        width: None,
        height: None,
    }
}

#[test]
fn root_frames_sort_numerically_then_id_and_exclude_nested_and_other_pages() {
    let mut snapshot = snapshot();
    snapshot.nodes = vec![
        node("page", None, NodeKind::Page, 0, json!({})),
        node("ten", Some("page"), NodeKind::Frame, 10, json!({})),
        node("b", Some("page"), NodeKind::Frame, 2, json!({})),
        node("a", Some("page"), NodeKind::Frame, 2, json!({})),
        node("nested", Some("a"), NodeKind::Frame, 0, json!({})),
        node("other", Some("else"), NodeKind::Frame, 0, json!({})),
        node("rect", Some("page"), NodeKind::Rectangle, 0, json!({})),
    ];
    let frames = query().frames(&snapshot).unwrap();
    assert_eq!(
        frames
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "ten"]
    );
}

#[test]
fn dimensions_match_authored_numeric_positive_fallback_and_shared_viewport() {
    let mut query = query();
    let frame = node(
        "f",
        Some("page"),
        NodeKind::Frame,
        0,
        json!({"width":321.5,"height":123}),
    );
    assert_eq!(
        (query.request(&frame).width, query.request(&frame).height),
        (321.5, 123.0)
    );
    for props in [
        json!({}),
        json!({"width":"fill","height":"hug"}),
        json!({"width":0,"height":-2}),
    ] {
        let frame = node("f", Some("page"), NodeKind::Frame, 0, props);
        assert_eq!(
            (query.request(&frame).width, query.request(&frame).height),
            (800.0, 600.0)
        );
    }
    query.width = Some(400.0);
    assert!(query.validate().is_err());
    query.height = Some(300.0);
    query.validate().unwrap();
    assert_eq!(
        (query.request(&frame).width, query.request(&frame).height),
        (400.0, 300.0)
    );
}

#[test]
fn invalid_revision_page_offset_limit_and_viewport_are_explicit_errors() {
    let mut snapshot = snapshot();
    snapshot
        .nodes
        .push(node("page", None, NodeKind::Page, 0, json!({})));
    let mut query = query();
    assert!(query.frames(&snapshot).unwrap().is_empty());
    query.revision = 6;
    assert_eq!(
        query.frames(&snapshot).unwrap_err().value()["error"]["code"],
        "REVISION_UNAVAILABLE"
    );
    query.revision = 7;
    query.page_id = "missing".into();
    assert!(query.frames(&snapshot).is_err());
    query.page_id = "page".into();
    query.offset = 1;
    assert!(query.frames(&snapshot).is_err());
    for limit in [0, 5, usize::MAX] {
        query.limit = limit;
        assert!(query.validate().is_err());
    }
    query.limit = 4;
    for width in [0.0, -1.0, f64::NAN, f64::INFINITY, 100001.0] {
        query.width = Some(width);
        query.height = Some(600.0);
        assert!(query.validate().is_err());
    }
}
