mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::json;

#[test]
fn typed_refs_resolve_and_upsert_invalidates_literal() {
    let (_temp, _store, p) = setup();
    let mut ops = vec![
        Operation::UpsertToken {
            token: Token {
                id: "brand".into(),
                name: "Brand".into(),
                token_type: "color".into(),
                value: json!("#112233"),
            },
        },
        Operation::UpsertStyle {
            style: Style {
                id: "surface".into(),
                name: "Surface".into(),
                kind: "fill".into(),
                value: json!({"tokenId":"brand"}),
            },
        },
    ];
    ops.extend(initial());
    ops.push(Operation::UpdateNode {
        id: "rect".into(),
        props: json!({"fill":{"styleId":"surface"}}),
    });
    p.apply_batch(request(&p, 0, "refs", ops)).unwrap();
    let s = p.snapshot().unwrap();
    assert_eq!(
        resolved_nodes(&s)
            .unwrap()
            .iter()
            .find(|n| n.id == "rect")
            .unwrap()
            .props["fill"],
        "#112233"
    );
    p.apply_batch(request(
        &p,
        1,
        "token-change",
        vec![Operation::UpsertToken {
            token: Token {
                id: "brand".into(),
                name: "Brand".into(),
                token_type: "color".into(),
                value: json!("#445566"),
            },
        }],
    ))
    .unwrap();
    assert_eq!(
        resolved_nodes(&p.snapshot().unwrap())
            .unwrap()
            .iter()
            .find(|n| n.id == "rect")
            .unwrap()
            .props["fill"],
        "#445566"
    );
    let invalid = Operation::UpsertToken {
        token: Token {
            id: "brand".into(),
            name: "Brand".into(),
            token_type: "number".into(),
            value: json!(20),
        },
    };
    assert_eq!(
        p.apply_batch(request(&p, 2, "wrong-type", vec![invalid]))
            .unwrap_err()
            .code,
        "VALIDATION_FAILED"
    );
    assert_eq!(p.snapshot().unwrap().revision, 2);
}

#[test]
fn reject_bad_schema_forward_refs_and_unsupported_layout() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    for (i, props) in [
        json!({"width":"50%"}),
        json!({"fill":{"tokenId":"missing"}}),
        json!({"layout":{"mode":"grid"}}),
        json!({"opacity":2}),
        json!({"width":null}),
        json!({"minWidth":200,"maxWidth":100}),
        json!({"unknown":true}),
    ]
    .into_iter()
    .enumerate()
    {
        let req = request(
            &p,
            1,
            &format!("bad-{i}"),
            vec![Operation::UpdateNode {
                id: "rect".into(),
                props,
            }],
        );
        assert_eq!(p.apply_batch(req).unwrap_err().code, "VALIDATION_FAILED");
    }
    let ops = vec![
        create(
            "late-child",
            NodeKind::Rectangle,
            Some("late-parent"),
            json!({"width":10}),
        ),
        create("late-parent", NodeKind::Group, Some("frame"), json!({})),
    ];
    assert_eq!(
        p.apply_batch(request(&p, 1, "forward", ops))
            .unwrap_err()
            .op_index,
        Some(0)
    );
    let ops = vec![
        create(
            "hug",
            NodeKind::Group,
            Some("frame"),
            json!({"width":"hug","height":20}),
        ),
        create(
            "fill",
            NodeKind::Rectangle,
            Some("hug"),
            json!({"width":"fill","height":20}),
        ),
    ];
    assert_eq!(
        p.apply_batch(request(&p, 1, "ambiguous", ops))
            .unwrap_err()
            .code,
        "VALIDATION_FAILED"
    );
    assert_eq!(p.snapshot().unwrap().revision, 1);
}

#[test]
fn styles_are_primitives_not_recursive_aliases() {
    let (_temp, _store, p) = setup();
    let s = Style {
        id: "style".into(),
        name: "Style".into(),
        kind: "fill".into(),
        value: json!("#fff"),
    };
    p.apply_batch(request(
        &p,
        0,
        "style",
        vec![Operation::UpsertStyle { style: s.clone() }],
    ))
    .unwrap();
    let mut alias = s;
    alias.value = json!({"styleId":"style"});
    assert_eq!(
        p.apply_batch(request(
            &p,
            1,
            "alias",
            vec![Operation::UpsertStyle { style: alias }]
        ))
        .unwrap_err()
        .code,
        "VALIDATION_FAILED"
    );
}
