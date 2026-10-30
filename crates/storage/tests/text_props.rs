mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

fn style(id: &str, extra: Value) -> Operation {
    let mut value = json!({"fontId":"font_bundled","fontSize":14,"lineHeight":20});
    value
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    Operation::UpsertStyle {
        style: Style {
            id: id.into(),
            name: id.into(),
            kind: "text".into(),
            value,
        },
    }
}
fn token(value: Value) -> Operation {
    Operation::UpsertToken {
        token: Token {
            id: "tracking".into(),
            name: "tracking".into(),
            token_type: "number".into(),
            value,
        },
    }
}

#[test]
fn typography_survives_style_theme_instance_snapshot_and_bundle() {
    let (_temp, store, p) = setup();
    let mut ops = vec![
        token(json!({"default":-0.5,"themes":{"wide":2.25}})),
        style(
            "type",
            json!({"letterSpacing":{"tokenId":"tracking"},"textWrap":"nowrap","textAlign":"center","textDecoration":"underline","textTransform":"uppercase"}),
        ),
        style(
            "alternate",
            json!({"textAlign":"right","textTransform":"capitalize","textDecoration":"line-through","letterSpacing":0}),
        ),
    ];
    ops.extend(initial());
    ops.extend([
        create(
            "master",
            NodeKind::Frame,
            Some("page"),
            json!({"component":{"name":"Title"},"width":200,"height":80}),
        ),
        create(
            "label",
            NodeKind::Text,
            Some("master"),
            json!({"text":"Straße tiếng Việt","styleId":"type"}),
        ),
        create(
            "one",
            NodeKind::Instance,
            Some("frame"),
            json!({"componentId":"master"}),
        ),
        create(
            "two",
            NodeKind::Instance,
            Some("frame"),
            json!({"componentId":"master","overrides":{"label":{"styleId":"alternate"}}}),
        ),
    ]);
    p.apply_batch(request(&p, 0, "type", ops)).unwrap();
    let before = p.snapshot().unwrap();
    for (theme, spacing) in [("default", -0.5), ("wide", 2.25)] {
        let resolved = resolved_expanded_nodes(&before, theme).unwrap();
        let props = |id| &resolved.iter().find(|n| n.id == id).unwrap().props;
        assert_eq!(props("one/label")["letterSpacing"], spacing);
        assert_eq!(props("one/label")["text"], "Straße tiếng Việt");
        assert_eq!(props("one/label")["textWrap"], "nowrap");
        assert_eq!(props("one/label")["textTransform"], "uppercase");
        assert_eq!(props("two/label")["letterSpacing"], 0);
        assert_eq!(props("two/label")["textAlign"], "right");
        assert_eq!(props("two/label")["textDecoration"], "line-through");
    }
    let copy = store
        .import_bundle("copy", &p.export_bundle().unwrap().bytes)
        .unwrap()
        .snapshot()
        .unwrap();
    assert_eq!(before.nodes, copy.nodes);
    assert_eq!(before.styles, copy.styles);
    assert_eq!(before.tokens, copy.tokens);
    assert_eq!(p.snapshot().unwrap(), before);
}

#[test]
fn invalid_text_values_and_late_token_branches_rollback_atomically() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let before = p.snapshot().unwrap();
    for extra in [
        json!({"letterSpacing":"0.1em"}),
        json!({"letterSpacing":1001}),
        json!({"letterSpacing":-1001}),
        json!({"textWrap":true}),
        json!({"textWrap":"balance"}),
        json!({"textAlign":"justify"}),
        json!({"textDecoration":"underline line-through"}),
        json!({"textTransform":"titlecase"}),
    ] {
        for operation in [
            Operation::UpdateNode {
                id: "text".into(),
                props: extra.clone(),
            },
            style("bad", extra),
        ] {
            let result = p.apply_batch(request(
                &p,
                1,
                "invalid",
                vec![
                    Operation::UpdateNode {
                        id: "rect".into(),
                        props: json!({"name":"must rollback"}),
                    },
                    operation,
                ],
            ));
            assert_eq!(result.unwrap_err().code, "VALIDATION_FAILED");
            assert_eq!(p.snapshot().unwrap(), before);
        }
    }
    let result = p.apply_batch(request(
        &p,
        1,
        "branch",
        vec![
            token(json!({"default":0,"themes":{"bad":1001}})),
            style("branch", json!({"letterSpacing":{"tokenId":"tracking"}})),
        ],
    ));
    assert!(result.is_err());
    assert_eq!(p.snapshot().unwrap(), before);
    let bad_state = Operation::UpdateNode {
        id: "text".into(),
        props: json!({"states":{"hover":{"letterSpacing":1}}}),
    };
    assert!(p
        .apply_batch(request(&p, 1, "state", vec![bad_state]))
        .is_err());
    assert_eq!(p.snapshot().unwrap(), before);
}
