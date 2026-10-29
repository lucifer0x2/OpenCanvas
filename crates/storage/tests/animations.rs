mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};
use std::sync::Arc;

fn project() -> (tempfile::TempDir, Store, Arc<Project>) {
    let (temp, store, p) = setup();
    let mut ops = initial();
    for (id, value) in [
        ("color-brand", json!("#1a7aff")),
        ("motion-fast", json!(200)),
    ] {
        let token_type = if id.starts_with("color") {
            "color"
        } else {
            "number"
        };
        ops.push(Operation::UpsertToken {
            token: Token {
                id: id.into(),
                name: id.into(),
                token_type: token_type.into(),
                value,
            },
        });
    }
    p.apply_batch(request(&p, 0, "initial", ops)).unwrap();
    (temp, store, p)
}

fn card(p: &Project, key: &str, animations: Value) -> Result<BatchResult> {
    let props = json!({"width":100,"height":40,"fill":"#1a7aff","animations":animations});
    let op = create("card", NodeKind::Rectangle, Some("frame"), props);
    p.apply_batch(request(p, 1, key, vec![op]))
}

const ENTER: &str = r#"{"name":"enter","duration":400,"easing":"ease-out","keyframes":[{"offset":0,"opacity":0,"translateY":12}]}"#;

#[test]
fn authored_animations_read_back_verbatim() {
    let (_t, _s, p) = project();
    let animations = json!([serde_json::from_str::<Value>(ENTER).unwrap()]);
    card(&p, "enter", animations.clone()).unwrap();
    let snapshot = p.snapshot().unwrap();
    let node = snapshot.nodes.iter().find(|n| n.id == "card").unwrap();
    assert_eq!(node.props["animations"], animations);
}

#[test]
fn valid_animations_commit() {
    let enter: Value = serde_json::from_str(ENTER).unwrap();
    let cases = [
        json!([enter,
            {"name":"lift","on":"hover","duration":200,"easing":"ease-out","keyframes":[{"scale":1.02}]},
            {"name":"pulse","duration":800,"iterations":"infinite","direction":"alternate","keyframes":[{"offset":1,"fill":{"tokenId":"color-brand"}}]}]),
        json!([{"name":"fade","duration":300,"keyframes":[{"opacity":0},{"opacity":0.5},{"opacity":1}]}]),
        json!([{"name":"tick","duration":800,"easing":"steps(4, end)","keyframes":[{"offset":0,"rotate":0,"easing":"cubic-bezier(0.34,1.56,0.64,1)"},{"rotate":90}]}]),
        json!([{"name":"spin","duration":{"tokenId":"motion-fast"},"iterations":2.5,"keyframes":[{"rotate":360}]}]),
        json!([]),
    ];
    for (i, animations) in cases.into_iter().enumerate() {
        let (_t, _s, p) = project();
        card(&p, &format!("valid-{i}"), animations.clone())
            .unwrap_or_else(|e| panic!("case {i}: {e:?}"));
    }
}

fn anim(extra: Value) -> Value {
    let mut base = json!({"name":"a","duration":400,"keyframes":[{"opacity":0}]});
    base.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    base
}

#[test]
fn invalid_animations_name_the_field() {
    let p0 = "props.animations[0]";
    let nine: Vec<Value> = (0..9)
        .map(|i| json!({"name":format!("a{i}"),"duration":400,"keyframes":[{"scale":1}]}))
        .collect();
    let cases: Vec<(&str, Value, String, Option<&str>)> = vec![
        (
            "a",
            json!([anim(json!({"trigger":"hover"}))]),
            format!("{p0}.trigger"),
            Some("on"),
        ),
        (
            "b",
            json!([anim(json!({"keyframes":[{"y":4}]}))]),
            format!("{p0}.keyframes[0].y"),
            Some("translateY"),
        ),
        (
            "c",
            json!([anim(json!({"keyframes":[{"rotation":4}]}))]),
            format!("{p0}.keyframes[0].rotation"),
            Some("rotate"),
        ),
        (
            "d",
            json!([anim(json!({"keyframes":[{"t":0,"opacity":0}]}))]),
            format!("{p0}.keyframes[0].t"),
            Some("offset"),
        ),
        (
            "e",
            json!([anim(json!({"fill":"forwards"}))]),
            format!("{p0}.fill"),
            Some("fillMode"),
        ),
        (
            "f",
            json!([anim(
                json!({"keyframes":[{"offset":0.6,"opacity":0},{"offset":0.4,"opacity":1}]})
            )]),
            format!("{p0}.keyframes[1].offset"),
            None,
        ),
        (
            "g",
            json!([anim(json!({})), anim(json!({"keyframes":[{"scale":2}]}))]),
            "props.animations[1].name".into(),
            None,
        ),
        (
            "h",
            json!([anim(json!({})), anim(json!({"name":"b"}))]),
            "props.animations[1].keyframes[0].opacity".into(),
            None,
        ),
        (
            "i",
            json!([anim(
                json!({"keyframes":[{"translateX":1},{"translateY":2}]})
            )]),
            format!("{p0}.keyframes[0].translateY"),
            None,
        ),
        (
            "j",
            json!([anim(
                json!({"on":"hover","keyframes":[{"opacity":0},{"opacity":1}]})
            )]),
            format!("{p0}.keyframes"),
            None,
        ),
        (
            "k",
            json!([anim(json!({"on":"hover","iterations":2}))]),
            format!("{p0}.iterations"),
            None,
        ),
        (
            "l",
            json!([anim(json!({"easing":"cubic-bezier(1.2,0,0,1)"}))]),
            format!("{p0}.easing"),
            None,
        ),
        (
            "m",
            json!([anim(json!({"duration":0.4}))]),
            format!("{p0}.duration"),
            None,
        ),
        (
            "n",
            json!([anim(json!({"keyframes":[{"opacity":1.5}]}))]),
            format!("{p0}.keyframes[0].opacity"),
            None,
        ),
        ("o", Value::Array(nine), "props.animations".into(), None),
        (
            "p",
            json!([anim(json!({"keyframes":[{"offset":1}]}))]),
            format!("{p0}.keyframes[0]"),
            None,
        ),
    ];
    for (case, animations, field, suggestion) in cases {
        let (_t, _s, p) = project();
        let e = card(&p, case, animations).unwrap_err();
        assert_eq!(e.code, "VALIDATION_FAILED", "{case}");
        assert_eq!(
            e.field.as_deref(),
            Some(field.as_str()),
            "{case}: {}",
            e.message
        );
        let node = e.node.as_deref().unwrap();
        assert_eq!(node.node_id.as_deref(), Some("card"), "{case}");
        assert_eq!(node.node_kind.as_deref(), Some("rectangle"), "{case}");
        assert_eq!(node.suggestion.as_deref(), suggestion, "{case}");
        let listed = |table: &[AnimationKey]| table.iter().map(|k| k.key.to_string()).collect();
        match case {
            "a" => assert_eq!(node.allowed_keys, Some(listed(ANIMATION_KEYS))),
            "b" => assert_eq!(node.allowed_keys, Some(listed(KEYFRAME_KEYS))),
            "j" => assert!(e.message.contains("one keyframe"), "{}", e.message),
            "m" => assert!(e.message.contains("milliseconds"), "{}", e.message),
            _ => {}
        }
    }
}

#[test]
fn animations_on_a_page_are_rejected() {
    let (_t, _s, p) = project();
    let props = json!({"name":"P2","animations":[anim(json!({}))]});
    let op = create("p2", NodeKind::Page, None, props);
    let e = p.apply_batch(request(&p, 1, "page", vec![op])).unwrap_err();
    assert_eq!(e.field.as_deref(), Some("props.animations"));
    assert_eq!(
        e.node.as_deref().unwrap().node_kind.as_deref(),
        Some("page")
    );
}

#[test]
fn animation_tokens_resolve_per_theme_and_check_their_type() {
    let (_t, _s, p) = project();
    let dark = Operation::UpsertToken {
        token: Token {
            id: "color-brand".into(),
            name: "color-brand".into(),
            token_type: "color".into(),
            value: json!({"default":"#1a7aff","themes":{"dark":"#000000"}}),
        },
    };
    p.apply_batch(request(&p, 1, "dark", vec![dark])).unwrap();
    let authored = json!([{"name":"pulse","duration":{"tokenId":"motion-fast"},
        "keyframes":[{"fill":{"tokenId":"color-brand"}}]}]);
    let op = create(
        "card",
        NodeKind::Rectangle,
        Some("frame"),
        json!({"width":10,"height":10,"animations":authored}),
    );
    p.apply_batch(request(&p, 2, "tokens", vec![op])).unwrap();
    let snapshot = p.snapshot().unwrap();
    let resolved = |theme| {
        let nodes = resolved_nodes_for_theme(&snapshot, theme).unwrap();
        nodes.into_iter().find(|n| n.id == "card").unwrap().props["animations"][0].clone()
    };
    assert_eq!(resolved("default")["duration"], json!(200));
    assert_eq!(
        resolved("default")["keyframes"][0]["fill"],
        json!("#1a7aff")
    );
    assert_eq!(resolved("dark")["keyframes"][0]["fill"], json!("#000000"));
    let card = snapshot.nodes.iter().find(|n| n.id == "card").unwrap();
    assert_eq!(card.props["animations"], authored);

    for (key, fill) in [("wrong-type", "motion-fast"), ("missing", "nope")] {
        let bad = json!([{"name":"x","duration":400,"keyframes":[{"fill":{"tokenId":fill}}]}]);
        let op = create(
            "bad",
            NodeKind::Rectangle,
            Some("frame"),
            json!({"width":1,"height":1,"animations":bad}),
        );
        let e = p.apply_batch(request(&p, 3, key, vec![op])).unwrap_err();
        assert_eq!(
            e.field.as_deref(),
            Some("props.animations[0].keyframes[0].fill"),
            "{key}"
        );
        assert_eq!(e.node.as_deref().unwrap().node_id.as_deref(), Some("bad"));
    }
}
