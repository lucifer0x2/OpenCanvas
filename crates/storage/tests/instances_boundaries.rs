mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::json;

#[test]
fn instances_nested_depth_eight_allowed_nine_rejected_and_indirect_cycle() {
    let (_temp, _store, p) = setup();
    let mut ops = vec![
        create("page", NodeKind::Page, None, json!({})),
        create(
            "frame",
            NodeKind::Frame,
            Some("page"),
            json!({"width":800,"height":600}),
        ),
    ];
    for i in 0..9 {
        ops.push(create(
            &format!("master{i}"),
            NodeKind::Frame,
            Some("page"),
            json!({"component":{"name":format!("Level{i}")},"width":100,"height":100}),
        ));
    }
    for i in 0..7 {
        ops.push(create(
            &format!("nested{i}"),
            NodeKind::Instance,
            Some(&format!("master{i}")),
            json!({"componentId":format!("master{}", i+1)}),
        ));
    }
    ops.push(create(
        "instance",
        NodeKind::Instance,
        Some("frame"),
        json!({"componentId":"master0"}),
    ));
    p.apply_batch(request(&p, 0, "eight", ops)).unwrap();
    let nodes = instances::expand(&p.snapshot().unwrap().nodes).unwrap();
    assert!(nodes
        .iter()
        .any(|n| n.id == "instance/nested0/nested1/nested2/nested3/nested4/nested5/nested6"));
    let nine = create(
        "nested7",
        NodeKind::Instance,
        Some("master7"),
        json!({"componentId":"master8"}),
    );
    assert_eq!(
        p.apply_batch(request(&p, 1, "nine", vec![nine]))
            .unwrap_err()
            .code,
        "INSTANCE_DEPTH"
    );
    let cycle = create(
        "cycle",
        NodeKind::Instance,
        Some("master1"),
        json!({"componentId":"master0"}),
    );
    assert_eq!(
        p.apply_batch(request(&p, 1, "cycle", vec![cycle]))
            .unwrap_err()
            .code,
        "INSTANCE_CYCLE"
    );
}

#[test]
fn instances_virtual_parent_is_readonly_and_override_refs_are_validated_for_every_theme() {
    let (_temp, _store, p) = setup();
    let mut ops = initial();
    ops.extend([
        create("master", NodeKind::Frame, Some("page"), json!({"component":{"name":"Master"},"width":200,"height":100})),
        create("label", NodeKind::Text, Some("master"), json!({"text":"base","fontId":"font_bundled","fontSize":14,"lineHeight":20})),
        Operation::UpsertToken { token:Token { id:"ink".into(), name:"Ink".into(), token_type:"color".into(), value:json!({"default":"#f00","themes":{"dark":"#0f0"}}) } },
        create("instance", NodeKind::Instance, Some("frame"), json!({"componentId":"master","overrides":{"label":{"text":"override","fill":{"tokenId":"ink"},"visible":false}}})),
    ]);
    p.apply_batch(request(&p, 0, "valid", ops)).unwrap();
    for (theme, color) in [("default", "#f00"), ("dark", "#0f0")] {
        let expanded = resolved_expanded_nodes(&p.snapshot().unwrap(), theme).unwrap();
        let label = expanded.iter().find(|n| n.id == "instance/label").unwrap();
        assert_eq!(label.props["fill"], color);
        assert_eq!(label.props["opacity"], 0);
    }
    let invalid = create(
        "bad",
        NodeKind::Rectangle,
        Some("instance/label"),
        json!({}),
    );
    assert_eq!(
        p.apply_batch(request(&p, 1, "parent", vec![invalid]))
            .unwrap_err()
            .code,
        "INSTANCE_CHILD_READONLY"
    );
    let invalid = Operation::UpdateNode {
        id: "instance".into(),
        props: json!({"overrides":{"label":{"fill":{"tokenId":"missing"}}}}),
    };
    assert!(p
        .apply_batch(request(&p, 1, "missing", vec![invalid]))
        .unwrap_err()
        .field
        .unwrap()
        .starts_with("props.overrides.label."));
}

#[test]
fn instances_expanded_node_budget_and_component_error_cap_are_atomic() {
    let (_temp, _store, p) = setup();
    let mut ops = vec![
        create("page", NodeKind::Page, None, json!({})),
        create(
            "frame",
            NodeKind::Frame,
            Some("page"),
            json!({"width":800,"height":600}),
        ),
        create(
            "master",
            NodeKind::Frame,
            Some("page"),
            json!({"component":{"name":"Large"},"width":100,"height":100}),
        ),
    ];
    for i in 0..200 {
        ops.push(create(
            &format!("child{i}"),
            NodeKind::Rectangle,
            Some("master"),
            json!({"width":1,"height":1}),
        ));
    }
    for i in 0..98 {
        ops.push(create(
            &format!("instance{i}"),
            NodeKind::Instance,
            Some("frame"),
            json!({"componentId":"master"}),
        ));
    }
    p.apply_batch(request(&p, 0, "under-budget", ops)).unwrap();
    let before = p.snapshot().unwrap();
    assert!(instances::expand(&before.nodes).unwrap().len() < MAX_NODES);
    let extra = create(
        "instance98",
        NodeKind::Instance,
        Some("frame"),
        json!({"componentId":"master"}),
    );
    assert_eq!(
        p.apply_batch(request(&p, 1, "over-budget", vec![extra]))
            .unwrap_err()
            .code,
        "INSTANCE_LIMIT"
    );
    let error = p
        .apply_batch(request(
            &p,
            1,
            "delete-master",
            vec![Operation::DeleteSubtree {
                id: "master".into(),
            }],
        ))
        .unwrap_err();
    assert_eq!(error.code, "COMPONENT_IN_USE");
    let context = error.node.unwrap();
    assert_eq!(context.count, Some(98));
    assert_eq!(context.instance_ids.unwrap().len(), 20);
    assert_eq!(p.snapshot().unwrap(), before);
}
