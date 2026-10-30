//! Paint props (radius, gradients, shadows): authored storage, refs and atomic rejection.
mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};
#[test]
fn accepts_tokens_styles_shorthand_gradients_shadows_preserving_authored() {
    let (_t, _s, p) = setup();
    let mut ops = initial();
    for (id, kind, value) in [
        (
            "ink",
            "color",
            json!({"default":"#6366f1","themes":{"night":"#eee"}}),
        ),
        ("r", "dimension", json!(12)),
        ("offset", "number", json!(4)),
    ] {
        ops.push(Operation::UpsertToken {
            token: Token {
                id: id.into(),
                name: id.into(),
                token_type: kind.into(),
                value,
            },
        });
    }
    ops.push(Operation::UpsertStyle {
        style: Style {
            id: "g".into(),
            name: "g".into(),
            kind: "fill".into(),
            value: json!({"type":"linear","stops":[[0,{"tokenId":"ink"}],[1,"#0000"]]}),
        },
    });
    let authored = json!({"radius":[{"tokenId":"r"},0],"fill":{"styleId":"g"},"stroke":{"width":[1,0],"color":[{"tokenId":"ink"},"#0000"]},"shadow":[{"y":{"tokenId":"offset"},"blur":{"tokenId":"r"},"color":{"tokenId":"ink"}}],"states":{"hover":{"radius":[1,2,3],"fill":{"type":"radial","stops":[[0,"#fff"],[1,"#000"]]}}}});
    ops.push(Operation::UpdateNode {
        id: "rect".into(),
        props: authored.clone(),
    });
    p.apply_batch(request(&p, 0, "paint", ops)).unwrap();
    let snapshot = p.snapshot().unwrap();
    let n = |nodes: Vec<Node>| nodes.into_iter().find(|n| n.id == "rect").unwrap();
    assert_eq!(
        n(snapshot.nodes.clone()).props["radius"],
        authored["radius"]
    );
    let r = n(resolved_nodes_for_theme(&snapshot, "night").unwrap());
    assert_eq!(r.props["radius"], json!([12, 0]));
    assert_eq!(r.props["stroke"]["color"], json!(["#eee", "#0000"]));
    assert_eq!(r.props["fill"]["stops"][0][1], "#eee");
    assert_eq!(
        r.props["shadow"][0],
        json!({"y":4,"blur":12,"color":"#eee"})
    );
}
#[test]
fn invalid_paints_are_atomic_with_useful_paths() {
    let (_t, _s, p) = setup();
    p.apply_batch(request(&p, 0, "init", initial())).unwrap();
    let cases: [(Value, &str); 12] = [
        (json!({"radius":[]}), "radius"),
        (json!({"radius":{"topLeft":8}}), "radius"),
        (
            json!({"stroke":{"width":{"top":1},"color":"#000"}}),
            "stroke.width",
        ),
        (
            json!({"fill":{"type":"linear","stops":[[0.8,"#fff"],[0.2,"#000"]]}}),
            "fill.stops[1]",
        ),
        (
            json!({"fill":{"type":"linear","stops":[{"position":0,"color":"#fff"},{"position":1,"color":"#000"}]}}),
            "fill.stops[0]",
        ),
        (
            json!({"fill":{"type":"radial","radius":0,"stops":[[0,"#fff"],[1,"#000"]]}}),
            "fill.radius",
        ),
        (json!({"shadow":{}}), "shadow"),
        (
            json!({"shadow":[{"blur":1001,"color":"#000"}]}),
            "shadow[0].blur",
        ),
        (json!({"shadow":[{}]}), "shadow[0].color"),
        (
            json!({"shadow":[{"offsetX":1,"color":"#000"}]}),
            "shadow[0].offsetX",
        ),
        (
            json!({"states":{"hover":{"shadow":[]}}}),
            "props.states.hover.shadow",
        ),
        (
            json!({"fill":{"tokenId":"missing","type":"linear","stops":[[0,"#fff"],[1,"#000"]]}}),
            "fill.tokenId",
        ),
    ];
    for (props, field) in cases {
        let e = p
            .apply_batch(request(
                &p,
                1,
                "bad",
                vec![Operation::UpdateNode {
                    id: "rect".into(),
                    props,
                }],
            ))
            .unwrap_err();
        assert_eq!(e.field.as_deref(), Some(field), "{e:?}");
        assert_eq!(p.snapshot().unwrap().revision, 1);
    }
    for props in [
        json!({"shadow":[]}),
        json!({"fill":{"type":"linear","stops":[[0,"#fff"],[1,"#000"]]}}),
    ] {
        let e = p
            .apply_batch(request(
                &p,
                1,
                "text",
                vec![Operation::UpdateNode {
                    id: "text".into(),
                    props,
                }],
            ))
            .unwrap_err();
        assert!(e
            .field
            .as_deref()
            .is_some_and(|s| s == "props.shadow" || s == "fill.type"));
    }
}
#[test]
fn a_valid_op_before_an_invalid_paint_never_commits() {
    let (_t, _s, p) = setup();
    p.apply_batch(request(&p, 0, "init", initial())).unwrap();
    let before = p.snapshot().unwrap();
    let ops = vec![
        Operation::UpdateNode {
            id: "rect".into(),
            props: json!({"radius":[4,8],"fill":{"type":"linear","stops":[[0,"#fff"],[1,"#000"]]}}),
        },
        create(
            "ok",
            NodeKind::Rectangle,
            Some("frame"),
            json!({"width":10,"height":10,"shadow":[{"y":2,"color":"#0004"}]}),
        ),
        Operation::UpdateNode {
            id: "rect".into(),
            props: json!({"shadow":[{"y":1,"color":"#0004"},{"spread":2,"color":"blue"}]}),
        },
    ];
    let e = p.apply_batch(request(&p, 1, "mixed", ops)).unwrap_err();
    assert_eq!(e.field.as_deref(), Some("shadow[1].color"), "{e:?}");
    assert_eq!(p.snapshot().unwrap().nodes, before.nodes);
    assert_eq!(p.snapshot().unwrap().revision, 1);
    let nine: Vec<Value> = (0..9).map(|_| json!({"color":"#000"})).collect();
    let e = p
        .apply_batch(request(
            &p,
            1,
            "nine",
            vec![Operation::UpdateNode {
                id: "rect".into(),
                props: json!({"shadow":nine}),
            }],
        ))
        .unwrap_err();
    assert_eq!(e.field.as_deref(), Some("shadow"));
    let seventeen: Vec<Value> = (0..17)
        .map(|i| json!([f64::from(i) / 16.0, "#000"]))
        .collect();
    let e = p
        .apply_batch(request(
            &p,
            1,
            "stops",
            vec![Operation::UpdateNode {
                id: "rect".into(),
                props: json!({"fill":{"type":"linear","stops":seventeen}}),
            }],
        ))
        .unwrap_err();
    assert_eq!(e.field.as_deref(), Some("fill.stops"));
}
