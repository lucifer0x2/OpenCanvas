mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

fn update(states: Value) -> Operation {
    Operation::UpdateNode {
        id: "rect".into(),
        props: json!({"states":states}),
    }
}

#[test]
fn states_resolve_themed_token_style_and_stroke_refs_without_changing_authored() {
    let (_temp, _store, p) = setup();
    let mut ops = vec![
        Operation::UpsertToken {
            token: Token {
                id: "brand".into(),
                name: "Brand".into(),
                token_type: "color".into(),
                value: json!({"default":"#123456","themes":{"night":"#abcdef"}}),
            },
        },
        Operation::UpsertToken {
            token: Token {
                id: "border".into(),
                name: "Border".into(),
                token_type: "dimension".into(),
                value: json!(3),
            },
        },
        Operation::UpsertStyle {
            style: Style {
                id: "paint".into(),
                name: "Paint".into(),
                kind: "fill".into(),
                value: json!({"tokenId":"brand"}),
            },
        },
    ];
    ops.extend(initial());
    p.apply_batch(request(&p, 0, "initial", ops)).unwrap();
    let states = json!({"hover":{"fill":{"tokenId":"brand"},"stroke":{"color":{"styleId":"paint"},"width":[0,1,{"tokenId":"border"},2]}},"pressed":{"opacity":0.4},"focus":{}});
    assert_eq!(
        p.apply_batch(request(&p, 1, "states", vec![update(states.clone())]))
            .unwrap()
            .committed_revision,
        2
    );
    let snapshot = p.snapshot().unwrap();
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .find(|n| n.id == "rect")
            .unwrap()
            .props["states"],
        states
    );
    for (theme, color) in [("default", "#123456"), ("night", "#abcdef")] {
        let resolved = resolved_nodes_for_theme(&snapshot, theme).unwrap();
        let rect = resolved.iter().find(|n| n.id == "rect").unwrap();
        assert_eq!(rect.props["states"]["hover"]["fill"], color);
        assert_eq!(rect.props["states"]["hover"]["stroke"]["color"], color);
        assert_eq!(
            rect.props["states"]["hover"]["stroke"]["width"],
            json!([0, 1, 3, 2])
        );
        assert_eq!(rect.props["fill"], "#1a7aff");
    }
}

#[test]
fn states_reject_unknown_names_keys_values_and_refs_atomically() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    for (states, field, allowed) in [
        (json!({"foo":{}}), "props.states.foo", Some("hover")),
        (
            json!({"hover":{"width":20}}),
            "props.states.hover.width",
            Some("fill"),
        ),
        (
            json!({"hover":{"states":{}}}),
            "props.states.hover.states",
            Some("fill"),
        ),
        (
            json!({"hover":{"fontWeight":700}}),
            "props.states.hover.fontWeight",
            Some("fill"),
        ),
        (
            json!({"hover":{"fill":{"tokenId":"missing"}}}),
            "props.states.hover.fill",
            None,
        ),
        (
            json!({"hover":{"opacity":2}}),
            "props.states.hover.opacity",
            None,
        ),
        (
            json!({"hover":{"radius":-1}}),
            "props.states.hover.radius",
            None,
        ),
        (
            json!({"hover":{"stroke":{"color":"red"}}}),
            "props.states.hover.stroke.color",
            None,
        ),
        (json!({"hover":[]}), "props.states.hover", None),
        (json!(null), "props.states", None),
    ] {
        let e = p
            .apply_batch(request(&p, 1, "invalid", vec![update(states)]))
            .unwrap_err();
        assert_eq!(e.field.as_deref(), Some(field), "{e:?}");
        if let Some(key) = allowed {
            assert!(e.node.unwrap().allowed_keys.unwrap().contains(&key.into()));
        }
        assert_eq!(p.snapshot().unwrap().revision, 1);
    }
    for (id, states, field) in [
        ("page", json!({}), "props.states"),
        (
            "text",
            json!({"hover":{"stroke":{"color":"#fff"}}}),
            "props.states.hover.stroke",
        ),
    ] {
        let e = p
            .apply_batch(request(
                &p,
                1,
                "scope",
                vec![Operation::UpdateNode {
                    id: id.into(),
                    props: json!({"states":states}),
                }],
            ))
            .unwrap_err();
        assert_eq!(e.field.as_deref(), Some(field));
    }
}

#[test]
fn states_validate_unused_themes_and_token_updates_and_clear_whole_object() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let token = |value| Operation::UpsertToken {
        token: Token {
            id: "alpha".into(),
            name: "Alpha".into(),
            token_type: "number".into(),
            value,
        },
    };
    p.apply_batch(request(
        &p,
        1,
        "valid",
        vec![
            token(json!(0.4)),
            update(json!({"disabled":{"opacity":{"tokenId":"alpha"}}})),
        ],
    ))
    .unwrap();
    let e = p
        .apply_batch(request(
            &p,
            2,
            "bad-theme",
            vec![token(json!({"default":0.4,"themes":{"night":2}}))],
        ))
        .unwrap_err();
    assert_eq!(e.field.as_deref(), Some("props.states.disabled.opacity"));
    assert_eq!(p.snapshot().unwrap().revision, 2);
    p.apply_batch(request(&p, 2, "clear", vec![update(json!({}))]))
        .unwrap();
    assert_eq!(
        p.snapshot()
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.id == "rect")
            .unwrap()
            .props["states"],
        json!({})
    );
}
