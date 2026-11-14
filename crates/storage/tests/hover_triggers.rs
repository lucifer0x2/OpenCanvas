mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

fn groups() -> Vec<Operation> {
    vec![
        create("card", NodeKind::Group, Some("frame"), json!({})),
        create("peer", NodeKind::Group, Some("frame"), json!({})),
        create(
            "other-frame",
            NodeKind::Frame,
            Some("page"),
            json!({"width":320,"height":200}),
        ),
        create("loose", NodeKind::Group, Some("page"), json!({})),
        Operation::UpsertToken {
            token: Token {
                id: "time".into(),
                name: "Time".into(),
                token_type: "number".into(),
                value: json!({"default":200,"themes":{"dark":300}}),
            },
        },
    ]
}

fn hover(source: Value) -> Value {
    json!({"name":"lift","on":"hover","triggerNodeId":source,
        "duration":{"tokenId":"time"},"delay":50,"keyframes":[{"scale":1.02}]})
}

fn target(parent: &str, animations: Value) -> Operation {
    create(
        "target",
        NodeKind::Rectangle,
        Some(parent),
        json!({"width":40,"height":40,"animations":animations}),
    )
}

#[test]
fn authored_group_and_root_frame_can_trigger_distinct_tracks() {
    let (_temp, _store, project) = setup();
    let mut ops = initial();
    ops.extend(groups());
    let mut fade = hover(json!("frame"));
    fade["name"] = json!("fade");
    fade["keyframes"] = json!([{"opacity":0.5}]);
    let animations = json!([hover(json!("card")), fade]);
    ops.push(target("card", animations.clone()));
    project
        .apply_batch(request(&project, 0, "seed", ops))
        .unwrap();
    let snapshot = project.snapshot().unwrap();
    let authored = snapshot.nodes.iter().find(|n| n.id == "target").unwrap();
    assert_eq!(authored.props["animations"], animations);
    let resolved = resolved_nodes_for_theme(&snapshot, "dark").unwrap();
    let target = resolved.iter().find(|n| n.id == "target").unwrap();
    assert_eq!(target.props["animations"][0]["triggerNodeId"], "card");
    assert_eq!(target.props["animations"][0]["duration"], 300);
    assert_eq!(target.props["animations"][0]["delay"], 50);
}

#[test]
fn invalid_sources_report_the_exact_target_field() {
    let (_temp, _store, project) = setup();
    let mut ops = initial();
    ops.extend(groups());
    project
        .apply_batch(request(&project, 0, "seed", ops))
        .unwrap();
    let mut load = hover(json!("card"));
    load.as_object_mut().unwrap().remove("on");
    let mut click = hover(json!("card"));
    click["on"] = json!("click");
    let cases = [
        ("card", load),
        ("card", click),
        ("card", hover(Value::Null)),
        ("card", hover(json!(42))),
        ("card", hover(json!({"tokenId":"time"}))),
        ("card", hover(json!("virtual/group"))),
        ("card", hover(json!("missing"))),
        ("card", hover(json!("target"))),
        ("card", hover(json!("peer"))),
        ("card", hover(json!("rect"))),
        ("card", hover(json!("page"))),
        ("card", hover(json!("other-frame"))),
        ("loose", hover(json!("loose"))),
    ];
    for (i, (parent, track)) in cases.into_iter().enumerate() {
        let err = project
            .apply_batch(request(
                &project,
                1,
                &format!("invalid-{i}"),
                vec![target(parent, json!([track]))],
            ))
            .unwrap_err();
        assert_eq!(err.code, "VALIDATION_FAILED", "case {i}: {err:?}");
        assert_eq!(
            err.field.as_deref(),
            Some("props.animations[0].triggerNodeId")
        );
    }
    assert_eq!(project.snapshot().unwrap().revision, 1);
    assert!(project.commits_after(1, 10).unwrap().is_empty());
}

#[test]
fn reparenting_validates_the_final_binding_and_rolls_back_an_invalid_batch() {
    let (_temp, _store, project) = setup();
    let mut ops = initial();
    ops.extend(groups());
    ops.push(target("card", json!([hover(json!("card"))])));
    project
        .apply_batch(request(&project, 0, "seed", ops))
        .unwrap();
    let reparent = |parent: &str| Operation::ReparentNode {
        id: "target".into(),
        parent_id: parent.into(),
        index: None,
    };
    let bind = |source: &str| Operation::UpdateNode {
        merge: false,
        id: "target".into(),
        props: json!({"animations":[hover(json!(source))]}),
    };

    let before = serde_json::to_value(project.snapshot().unwrap()).unwrap();
    let err = project
        .apply_batch(request(&project, 1, "move", vec![reparent("peer")]))
        .unwrap_err();
    assert_eq!(
        err.field.as_deref(),
        Some("props.animations[0].triggerNodeId")
    );
    assert_eq!(
        serde_json::to_value(project.snapshot().unwrap()).unwrap(),
        before
    );
    assert!(project.commits_after(1, 10).unwrap().is_empty());

    project
        .apply_batch(request(
            &project,
            1,
            "move",
            vec![reparent("peer"), bind("peer")],
        ))
        .unwrap();
    project
        .apply_batch(request(
            &project,
            2,
            "back",
            vec![bind("card"), reparent("card")],
        ))
        .unwrap();
    let after = project.snapshot().unwrap();
    assert_eq!(after.revision, 3);
    let node = after.nodes.iter().find(|n| n.id == "target").unwrap();
    assert_eq!(node.parent_id.as_deref(), Some("card"));
    assert_eq!(node.props["animations"][0]["triggerNodeId"], "card");
}
