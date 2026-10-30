//! Paint props through states, instance overrides, themes and bundle export/import.
mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::json;
#[test]
fn state_paint_is_validated_and_state_shadow_says_where_to_put_it() {
    let (_t, _s, p) = setup();
    p.apply_batch(request(&p, 0, "init", initial())).unwrap();
    let bad =
        json!({"states":{"hover":{"fill":{"type":"linear","stops":[[0.8,"#fff"],[0.2,"#000"]]}}}});
    let e = p
        .apply_batch(request(
            &p,
            1,
            "state-grad",
            vec![Operation::UpdateNode {
                id: "rect".into(),
                props: bad,
            }],
        ))
        .unwrap_err();
    assert_eq!(
        e.field.as_deref(),
        Some("props.states.hover.fill.stops[1]"),
        "{e:?}"
    );
    let e = p
        .apply_batch(request(
            &p,
            1,
            "state-shadow",
            vec![Operation::UpdateNode {
                id: "rect".into(),
                props: json!({"states":{"hover":{"shadow":[{"color":"#000"}]}}}),
            }],
        ))
        .unwrap_err();
    assert_eq!(e.field.as_deref(), Some("props.states.hover.shadow"));
    assert!(
        e.message.contains("not supported in interaction states")
            && e.message.contains("resting node"),
        "{}",
        e.message
    );
    let ok = json!({"states":{"pressed":{"radius":[2,6],"fill":{"type":"radial","center":[0.5,0],"stops":[[0,"#fff"],[1,"#0000"]]}}}});
    p.apply_batch(request(
        &p,
        1,
        "state-ok",
        vec![Operation::UpdateNode {
            id: "rect".into(),
            props: ok,
        }],
    ))
    .unwrap();
}
#[test]
fn instance_gradient_overrides_resolve_per_theme_and_bundle_roundtrips_paint() {
    let (_t, store, p) = setup();
    let mut ops = initial();
    ops.push(Operation::UpsertToken {
        token: Token {
            id: "ink".into(),
            name: "ink".into(),
            token_type: "color".into(),
            value: json!({"default":"#6366f1","themes":{"night":"#eeeeee"}}),
        },
    });
    ops.push(create("master",NodeKind::Frame,Some("page"),json!({"component":{"name":"Card"},"width":120,"height":60,"radius":[12,4],"shadow":[{"y":4,"blur":12,"color":{"tokenId":"ink"}},{"inset":true,"spread":1,"color":"#fff3"}],"stroke":{"color":["#000","#fff"],"width":[1,2]},"layout":{"mode":"free"}})));
    ops.push(create(
        "chip",
        NodeKind::Rectangle,
        Some("master"),
        json!({"width":40,"height":20,"fill":"#fff","radius":[8,0,8,0]}),
    ));
    let grad = json!({"type":"linear","angle":90,"stops":[[0,{"tokenId":"ink"}],[1,"#0000"]]});
    ops.push(create(
        "one",
        NodeKind::Instance,
        Some("frame"),
        json!({"componentId":"master","overrides":{"chip":{"fill":grad}}}),
    ));
    p.apply_batch(request(&p, 0, "inst", ops)).unwrap();
    let snapshot = p.snapshot().unwrap();
    let find = |nodes: &[Node], id: &str| nodes.iter().find(|n| n.id == id).unwrap().clone();
    let night = resolved_expanded_nodes(&snapshot, "night").unwrap();
    let chip = find(&night, "one/chip");
    assert_eq!(
        chip.props["fill"]["stops"],
        json!([[0, "#eeeeee"], [1, "#0000"]])
    );
    assert_eq!(chip.props["radius"], json!([8, 0, 8, 0]));
    assert_eq!(find(&night, "one").props["shadow"][0]["color"], "#eeeeee");
    let bad = json!({"componentId":"master","overrides":{"chip":{"fill":{"type":"conic","stops":[[0,"#fff"],[1,"#000"]]}}}});
    let e = p
        .apply_batch(request(
            &p,
            1,
            "bad-override",
            vec![Operation::UpdateNode {
                id: "one".into(),
                props: bad,
            }],
        ))
        .unwrap_err();
    assert_eq!(
        e.field.as_deref(),
        Some("props.overrides.chip.fill.type"),
        "{e:?}"
    );
    let e = p
        .apply_batch(request(
            &p,
            1,
            "shadow-override",
            vec![Operation::UpdateNode {
                id: "one".into(),
                props: json!({"overrides":{"chip":{"shadow":[]}}}),
            }],
        ))
        .unwrap_err();
    assert_eq!(
        e.field.as_deref(),
        Some("props.overrides.chip.shadow"),
        "{e:?}"
    );
    assert_eq!(p.snapshot().unwrap().revision, 1);
    let bundle = p.export_bundle().unwrap();
    let imported = store
        .import_bundle("copy", &bundle.bytes)
        .unwrap()
        .snapshot()
        .unwrap();
    assert_eq!(imported.nodes, snapshot.nodes);
    assert_eq!(
        find(&imported.nodes, "master").props["radius"],
        json!([12, 4])
    );
    assert_eq!(
        find(
            &resolved_expanded_nodes(&imported, "night").unwrap(),
            "one/chip"
        )
        .props["fill"],
        chip.props["fill"]
    );
}
