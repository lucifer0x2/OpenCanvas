//! Fill keyframes need a solid resting fill: a gradient (authored or from a fill style) would
//! hide the animated color in the HTML preview. Rejections are atomic and name the keyframe.
mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

fn gradient() -> Value {
    json!({"type":"linear","stops":[[0,"#ff0000"],[1,"#0000ff"]]})
}
fn fill_load() -> Value {
    json!([{"name":"tint","duration":400,"keyframes":[{"offset":0,"opacity":1},{"offset":1,"fill":"#00ff00"}]}])
}
fn update(id: &str, props: Value) -> Operation {
    Operation::UpdateNode {
        id: id.into(),
        props,
    }
}
fn fill_style(id: &str, value: Value) -> Operation {
    Operation::UpsertStyle {
        style: Style {
            id: id.into(),
            name: id.into(),
            kind: "fill".into(),
            value,
        },
    }
}
fn color_token() -> Operation {
    Operation::UpsertToken {
        token: Token {
            id: "brand".into(),
            name: "brand".into(),
            token_type: "color".into(),
            value: json!({"default":"#112233","themes":{"night":"#ddeeff"}}),
        },
    }
}
const KEYFRAME: &str = "props.animations[0].keyframes[1].fill";

/// Applies `ops` at the current revision and returns the error, asserting nothing committed.
fn rejected(p: &Project, ops: Vec<Operation>) -> StorageError {
    let before = p.snapshot().unwrap();
    let e = p
        .apply_batch(request(p, before.revision, "bad", ops))
        .unwrap_err();
    let after = p.snapshot().unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.nodes, before.nodes);
    assert_eq!(after.styles, before.styles);
    e
}
fn ready() -> (tempfile::TempDir, Store, std::sync::Arc<Project>) {
    let (t, s, p) = setup();
    let mut ops = initial();
    ops.push(fill_style("solid", json!("#445566")));
    ops.push(color_token());
    p.apply_batch(request(&p, 0, "init", ops)).unwrap();
    (t, s, p)
}

#[test]
fn gradient_resting_fill_rejects_fill_keyframes_wherever_the_gradient_comes_from() {
    let (_t, _s, p) = ready();
    let hover = json!([{"name":"glow","on":"hover","duration":200,"keyframes":[{"fill":{"tokenId":"brand"}}]}]);
    let cases: Vec<(Vec<Operation>, &str)> = vec![
        // Authored gradient, load animation.
        (
            vec![update(
                "rect",
                json!({"fill":gradient(),"animations":fill_load()}),
            )],
            KEYFRAME,
        ),
        // Authored radial gradient, hover animation with a token color.
        (
            vec![update(
                "rect",
                json!({"fill":{"type":"radial","stops":[[0,"#fff"],[1,"#000"]]},"animations":hover}),
            )],
            "props.animations[0].keyframes[0].fill",
        ),
        // Gradient from a fill style bound as `fill: {styleId}`.
        (
            vec![
                fill_style("grad", gradient()),
                update(
                    "rect",
                    json!({"fill":{"styleId":"grad"},"animations":fill_load()}),
                ),
            ],
            KEYFRAME,
        ),
        // Gradient from the node-level `styleId`, after a valid op in the same batch.
        (
            vec![
                update("rect", json!({"opacity":0.5})),
                fill_style("grad", gradient()),
                create(
                    "styled",
                    NodeKind::Rectangle,
                    Some("frame"),
                    json!({"width":8,"height":8,"styleId":"grad","animations":fill_load()}),
                ),
            ],
            KEYFRAME,
        ),
    ];
    for (i, (ops, field)) in cases.into_iter().enumerate() {
        let node = if i == 3 { "styled" } else { "rect" };
        let e = rejected(&p, ops);
        assert_eq!(e.code, "VALIDATION_FAILED", "{e:?}");
        assert_eq!(e.field.as_deref(), Some(field), "{e:?}");
        assert!(e.message.contains("solid resting fill"), "{e:?}");
        assert_eq!(e.node.as_ref().unwrap().node_id.as_deref(), Some(node));
    }
}

#[test]
fn turning_an_animated_nodes_style_into_a_gradient_rejects_the_whole_batch() {
    let (_t, _s, p) = ready();
    p.apply_batch(request(
        &p,
        1,
        "bind",
        vec![update(
            "rect",
            json!({"fill":{"styleId":"solid"},"animations":fill_load()}),
        )],
    ))
    .unwrap();
    let before = p.snapshot().unwrap();
    let ops = vec![
        create(
            "ok",
            NodeKind::Rectangle,
            Some("frame"),
            json!({"width":8,"height":8}),
        ),
        fill_style("solid", gradient()),
    ];
    let e = p.apply_batch(request(&p, 2, "restyle", ops)).unwrap_err();
    assert_eq!(e.field.as_deref(), Some(KEYFRAME), "{e:?}");
    let after = p.snapshot().unwrap();
    assert_eq!(after.revision, 2);
    assert_eq!(after.nodes, before.nodes);
    assert_eq!(after.styles, before.styles);
}

#[test]
fn solid_fills_tokens_and_gradient_motion_stay_valid() {
    let (_t, _s, p) = ready();
    let motion = json!([
        {"name":"enter","duration":400,"keyframes":[{"offset":0,"opacity":0,"translateY":12}]},
        {"name":"lift","on":"hover","duration":200,"keyframes":[{"scale":1.05,"rotate":2}]}
    ]);
    let ops = vec![
        // Gradient box with opacity/transform animations.
        update("rect", json!({"fill":gradient(),"animations":motion})),
        // Solid literal, solid style and themed color token resting fills with fill keyframes.
        create(
            "lit",
            NodeKind::Rectangle,
            Some("frame"),
            json!({"width":8,"height":8,"fill":"#ff0000","animations":fill_load()}),
        ),
        create(
            "sty",
            NodeKind::Frame,
            Some("frame"),
            json!({"width":8,"height":8,"fill":{"styleId":"solid"},"animations":fill_load()}),
        ),
        create(
            "tok",
            NodeKind::Group,
            Some("frame"),
            json!({"fill":{"tokenId":"brand"},"animations":fill_load()}),
        ),
        // A gradient only in a state does not hide the resting animation (states win anyway).
        create(
            "st",
            NodeKind::Rectangle,
            Some("frame"),
            json!({"width":8,"height":8,"fill":"#ff0000","states":{"hover":{"fill":gradient()}},"animations":fill_load()}),
        ),
    ];
    p.apply_batch(request(&p, 1, "ok", ops)).unwrap();
    let snapshot = p.snapshot().unwrap();
    for theme in ["default", "night"] {
        let nodes = resolved_nodes_for_theme(&snapshot, theme).unwrap();
        let get = |id: &str| nodes.iter().find(|n| n.id == id).unwrap().props.clone();
        assert_eq!(get("rect")["fill"]["type"], "linear");
        assert_eq!(get("rect")["animations"][1]["keyframes"][0]["scale"], 1.05);
        assert_eq!(
            get("lit")["animations"][0]["keyframes"][1]["fill"],
            "#00ff00"
        );
        assert!(get("tok")["fill"].is_string());
    }
}

#[test]
fn instances_cannot_combine_gradient_overrides_with_fill_animation() {
    let (_t, _s, p) = ready();
    let ops = vec![
        create(
            "card",
            NodeKind::Frame,
            Some("page"),
            json!({"component":{"name":"Card"},"width":120,"height":60,"layout":{"mode":"free"}}),
        ),
        create(
            "chip",
            NodeKind::Rectangle,
            Some("card"),
            json!({"width":20,"height":20,"fill":"#ff0000"}),
        ),
        create(
            "use",
            NodeKind::Instance,
            Some("frame"),
            json!({"componentId":"card","overrides":{"chip":{"fill":gradient()}}}),
        ),
    ];
    p.apply_batch(request(&p, 1, "card", ops)).unwrap();
    // Master side: an animated master child is rejected whatever its fill.
    let e = rejected(&p, vec![update("chip", json!({"animations":fill_load()}))]);
    assert_eq!(e.field.as_deref(), Some("props.component"), "{e:?}");
    let e = rejected(
        &p,
        vec![update(
            "chip",
            json!({"fill":gradient(),"animations":fill_load()}),
        )],
    );
    assert_eq!(e.field.as_deref(), Some(KEYFRAME), "{e:?}");
    // Instance side: an instance carries no animations of its own.
    let e = rejected(&p, vec![update("use", json!({"animations":fill_load()}))]);
    assert_eq!(e.field.as_deref(), Some("props.animations"), "{e:?}");
}
